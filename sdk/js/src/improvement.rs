//! Node projection of the API-owned human-governed configuration review.
//!
//! Each stage is linear: a transition consumes its handle even when the API
//! rejects that transition. Borrowed preparation is the exception: failure
//! leaves the explanation available for retry, and commit requires that exact
//! owner, not merely equal coordinates. See
//! [`pale_sdk::PaleApi::prepare_configuration_improvement_preview`].
//!
//! Retained comparison and profile queries reuse their existing projections.
//! These review methods return decisions without writing configuration or
//! establishing write authority.
//! Garbage collection does not choose a terminal disposition.

use crate::authoring::authoring_page_limit;
use crate::error::diagnostic_error;
use crate::{
    EffectiveProfileCompilation, OneShotPrivateInputs, Pale, PortableWorkspaceCompilation,
    PortableWorkspaceRequest, PortableWorkspaceSubjectInspection,
    PortableWorkspaceSubjectInspectionCursor, WorkspaceChangeRootPage, WorkspaceComparisonSummary,
    WorkspaceExplanation, WorkspaceImpactPage, WorkspaceSubject,
};
use napi::bindgen_prelude::{AsyncTask, Buffer};
use napi::{Env, Error, Result, Status, Task};
use napi_derive::napi;
use pale_sdk as api;
use std::sync::Arc;

/// Two absolute manifest locators and one common context for a fresh local review.
///
/// Candidate Package files are not read. Locators do not identify open editor
/// buffers, and capture does not promise an atomic snapshot across both roots.
#[napi(object, object_to_js = false)]
pub struct LocalConfigurationImprovementRequest {
    /// Baseline manifest; its Package source supplies the review.
    pub baseline_workspace_manifest: String,
    /// Candidate manifest; only workspace and private inputs are captured.
    pub candidate_workspace_manifest: String,
    /// Common logical project selector.
    pub project_id: String,
    /// Common logical execution-workspace selector.
    pub execution_workspace_id: String,
    /// Common private machine selector, not machine readiness.
    pub machine_id: String,
}

impl LocalConfigurationImprovementRequest {
    fn into_api(self) -> api::LocalConfigurationImprovementRequest {
        api::LocalConfigurationImprovementRequest::new(
            api::LocalWorkspaceLocator::new(self.baseline_workspace_manifest),
            api::LocalWorkspaceLocator::new(self.candidate_workspace_manifest),
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
        )
    }
}

/// Rust-owned local capture and preview work scheduled on the Node-API pool.
///
/// JavaScript borrows never enter the worker. The original API result is kept
/// until JavaScript-thread resolution, including canonical rejection details.
pub struct LocalConfigurationImprovementTask {
    api: Arc<api::PaleApi>,
    request: Option<api::LocalConfigurationImprovementRequest>,
}

#[napi]
impl Task for LocalConfigurationImprovementTask {
    type Output = std::result::Result<api::ConfigurationImprovementPreview, api::ApiError>;
    type JsValue = ConfigurationImprovementPreview;

    fn compute(&mut self) -> Result<Self::Output> {
        let request = self.request.take().ok_or_else(|| {
            Error::new(
                Status::GenericFailure,
                "local review task input was already consumed",
            )
        })?;
        Ok(self.api.preview_local_configuration_improvement(request))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output
            .map(|inner| ConfigurationImprovementPreview { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }
}

/// Closed API-owned review phase, never a machine-readiness flag.
#[napi]
pub enum ConfigurationImprovementPhase {
    /// An admitted baseline awaiting its exact profile explanation.
    Inspected,
    /// The exact baseline explanation retained for proposal or borrowed preparation.
    Explained,
    /// A captured candidate awaiting canonical compiler evaluation.
    Proposed,
    /// An admitted candidate awaiting the canonical impact proof.
    Evaluated,
    /// A diagnosed candidate awaiting a complete immutable preview.
    Diagnosed,
    /// A reviewable candidate; terminal disposition consumes it without effects.
    Previewed,
}
impl From<api::ConfigurationImprovementPhase> for ConfigurationImprovementPhase {
    fn from(value: api::ConfigurationImprovementPhase) -> Self {
        match value {
            api::ConfigurationImprovementPhase::Inspected => Self::Inspected,
            api::ConfigurationImprovementPhase::Explained => Self::Explained,
            api::ConfigurationImprovementPhase::Proposed => Self::Proposed,
            api::ConfigurationImprovementPhase::Evaluated => Self::Evaluated,
            api::ConfigurationImprovementPhase::Diagnosed => Self::Diagnosed,
            api::ConfigurationImprovementPhase::Previewed => Self::Previewed,
        }
    }
}

/// Explicit effect-free review outcome; accepted is not an effect grant.
#[napi]
pub enum ConfigurationImprovementDisposition {
    /// May be presented to a separate authority/effect gate.
    Accepted,
    /// The reviewer rejected the candidate.
    Rejected,
    /// The reviewer cancelled the review.
    Cancelled,
    /// An outer owner established that its own coordinate changed.
    Stale,
}
impl From<api::ConfigurationImprovementDisposition> for ConfigurationImprovementDisposition {
    fn from(value: api::ConfigurationImprovementDisposition) -> Self {
        match value {
            api::ConfigurationImprovementDisposition::Accepted => Self::Accepted,
            api::ConfigurationImprovementDisposition::Rejected => Self::Rejected,
            api::ConfigurationImprovementDisposition::Cancelled => Self::Cancelled,
            api::ConfigurationImprovementDisposition::Stale => Self::Stale,
        }
    }
}

/// Host misuse is distinct from a canonical API rejection.
fn consumed() -> Error {
    Error::new(
        Status::InvalidArg,
        "configuration-improvement handle has already been consumed",
    )
}

/// Borrow linear custody without advancing it.
fn borrow<T>(value: &Option<T>) -> Result<&T> {
    value.as_ref().ok_or_else(consumed)
}

/// Transfer linear custody exactly once, including failed API transitions.
fn take<T>(value: &mut Option<T>) -> Result<T> {
    value.take().ok_or_else(consumed)
}

/// An admitted baseline awaiting its exact profile explanation.
#[napi]
pub struct ConfigurationImprovementInspection {
    inner: Option<api::ConfigurationImprovementInspection>,
}
#[napi]
impl ConfigurationImprovementInspection {
    /// Current API-owned phase.
    ///
    /// # Errors
    ///
    /// Returns a host-argument error after this stage has been consumed.
    #[napi(getter)]
    pub fn phase(&self) -> Result<ConfigurationImprovementPhase> {
        Ok(borrow(&self.inner)?.phase().into())
    }
}

/// The exact baseline explanation retained for proposal or borrowed preparation.
#[napi]
pub struct ConfigurationImprovementExplanation {
    inner: Option<api::ConfigurationImprovementExplanation>,
}
#[napi]
impl ConfigurationImprovementExplanation {
    /// Current API-owned phase.
    ///
    /// # Errors
    ///
    /// Returns a host-argument error after this stage has been consumed.
    #[napi(getter)]
    pub fn phase(&self) -> Result<ConfigurationImprovementPhase> {
        Ok(borrow(&self.inner)?.phase().into())
    }
}

/// A captured candidate awaiting canonical compiler evaluation.
#[napi]
pub struct ConfigurationImprovementProposal {
    inner: Option<api::ConfigurationImprovementProposal>,
}
#[napi]
impl ConfigurationImprovementProposal {
    /// Current API-owned phase.
    ///
    /// # Errors
    ///
    /// Returns a host-argument error after this stage has been consumed.
    #[napi(getter)]
    pub fn phase(&self) -> Result<ConfigurationImprovementPhase> {
        Ok(borrow(&self.inner)?.phase().into())
    }
}

/// An admitted candidate awaiting the canonical impact proof.
#[napi]
pub struct ConfigurationImprovementEvaluation {
    inner: Option<api::ConfigurationImprovementEvaluation>,
}
#[napi]
impl ConfigurationImprovementEvaluation {
    /// Current API-owned phase.
    ///
    /// # Errors
    ///
    /// Returns a host-argument error after this stage has been consumed.
    #[napi(getter)]
    pub fn phase(&self) -> Result<ConfigurationImprovementPhase> {
        Ok(borrow(&self.inner)?.phase().into())
    }
}

/// A diagnosed candidate awaiting a complete immutable preview.
#[napi]
pub struct ConfigurationImprovementDiagnosis {
    inner: Option<api::ConfigurationImprovementDiagnosis>,
}
#[napi]
impl ConfigurationImprovementDiagnosis {
    /// Current API-owned phase.
    ///
    /// # Errors
    ///
    /// Returns a host-argument error after this stage has been consumed.
    #[napi(getter)]
    pub fn phase(&self) -> Result<ConfigurationImprovementPhase> {
        Ok(borrow(&self.inner)?.phase().into())
    }
}

/// A reviewable candidate; terminal disposition consumes it without effects.
#[napi]
pub struct ConfigurationImprovementPreview {
    inner: Option<api::ConfigurationImprovementPreview>,
}
#[napi]
impl ConfigurationImprovementPreview {
    /// Current API-owned phase.
    ///
    /// # Errors
    ///
    /// Returns a host-argument error after this stage has been consumed.
    #[napi(getter)]
    pub fn phase(&self) -> Result<ConfigurationImprovementPhase> {
        Ok(borrow(&self.inner)?.phase().into())
    }
}

/// Prepared candidate that exposes only its summary until exact-owner commit.
#[napi]
pub struct PreparedConfigurationImprovementPreview {
    inner: Option<api::PreparedConfigurationImprovementPreview>,
}

/// Fixed private inputs for a candidate that is already portably admitted.
///
/// No workspace documents are accepted here; preparation reuses the supplied
/// opaque candidate instead of recompiling its source.
#[napi(object, object_to_js = false)]
pub struct ConfigurationImprovementCandidateRequest {
    /// Selected logical project.
    pub project_id: String,
    /// Selected logical execution workspace.
    pub execution_workspace_id: String,
    /// Selected machine binding, not realization evidence.
    pub machine_id: String,
    /// Captured private source; no ambient filesystem access.
    pub private_inputs: OneShotPrivateInputs,
}
impl ConfigurationImprovementCandidateRequest {
    fn into_api(self) -> api::ConfigurationImprovementCandidateRequest {
        api::ConfigurationImprovementCandidateRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
        )
    }
}

/// Candidate source for the explicit propose/evaluate route.
///
/// Package source is absent: the API retains the baseline's admitted Package
/// owner. This request captures source but does not itself admit the candidate.
#[napi(object, object_to_js = false)]
pub struct ConfigurationImprovementProposalRequest {
    /// Selected logical project.
    pub project_id: String,
    /// Selected logical execution workspace.
    pub execution_workspace_id: String,
    /// Selected machine binding, not realization evidence.
    pub machine_id: String,
    /// Complete captured portable workspace candidate.
    pub workspace: PortableWorkspaceRequest,
    /// Captured private profile inputs.
    pub private_inputs: OneShotPrivateInputs,
}
impl ConfigurationImprovementProposalRequest {
    fn into_api(self) -> api::ConfigurationImprovementProposalRequest {
        api::ConfigurationImprovementProposalRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.workspace.into_api(),
            self.private_inputs.into_api(),
        )
    }
}

#[napi]
impl Pale {
    /// Capture two local source roots and return a fresh effect-free preview.
    ///
    /// The request is owned before returning the promise. The worker uses the
    /// SDK's capture and review owners; it neither resumes an editor review nor
    /// chooses a terminal decision. Acceptance still performs no source write.
    ///
    /// # Errors
    ///
    /// The promise rejects with the canonical API diagnostic for invalid
    /// locators, capture, selection, compilation or review input. This route
    /// introduces no cancellation or native-effect authority.
    #[napi]
    pub fn preview_local_configuration_improvement(
        &self,
        request: LocalConfigurationImprovementRequest,
    ) -> AsyncTask<LocalConfigurationImprovementTask> {
        AsyncTask::new(LocalConfigurationImprovementTask {
            api: Arc::clone(&self.api),
            request: Some(request.into_api()),
        })
    }

    /// Capture one admitted baseline without compiling or mutating it.
    #[napi]
    pub fn inspect_configuration_improvement(
        &self,
        baseline: &PortableWorkspaceCompilation,
    ) -> ConfigurationImprovementInspection {
        ConfigurationImprovementInspection {
            inner: Some(self.api.inspect_configuration_improvement(&baseline.inner)),
        }
    }

    /// Consume inspection and bind its exact effective-profile explanation.
    ///
    /// # Errors
    ///
    /// Returns a host error for consumed custody or the canonical API
    /// diagnostic for a different admitted frontier. Inspection is consumed.
    #[napi]
    pub fn explain_configuration_improvement(
        &self,
        env: Env,
        inspection: &mut ConfigurationImprovementInspection,
        profile: &EffectiveProfileCompilation,
    ) -> Result<ConfigurationImprovementExplanation> {
        self.api
            .explain_configuration_improvement(take(&mut inspection.inner)?, &profile.inner)
            .map(|inner| ConfigurationImprovementExplanation { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Consume explanation and capture candidate source without evaluating it.
    ///
    /// # Errors
    ///
    /// Returns consumed-handle misuse or canonical source-capture diagnostics.
    /// The explanation is consumed even if capture rejects the request.
    #[napi]
    pub fn propose_configuration_improvement(
        &self,
        env: Env,
        explanation: &mut ConfigurationImprovementExplanation,
        request: ConfigurationImprovementProposalRequest,
    ) -> Result<ConfigurationImprovementProposal> {
        self.api
            .propose_configuration_improvement(take(&mut explanation.inner)?, request.into_api())
            .map(|inner| ConfigurationImprovementProposal { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Admit the captured candidate through the canonical compiler.
    ///
    /// # Errors
    ///
    /// Returns consumed-handle misuse or the API-owned rejection diagnostic.
    /// The input stage is consumed on both success and API rejection.
    #[napi]
    pub fn evaluate_configuration_improvement(
        &self,
        env: Env,
        stage: &mut ConfigurationImprovementProposal,
    ) -> Result<ConfigurationImprovementEvaluation> {
        self.api
            .evaluate_configuration_improvement(take(&mut stage.inner)?)
            .map(|inner| ConfigurationImprovementEvaluation { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Derive the canonical change-root and impact proof.
    ///
    /// # Errors
    ///
    /// Returns consumed-handle misuse or the API-owned rejection diagnostic.
    /// The input stage is consumed on both success and API rejection.
    #[napi]
    pub fn diagnose_configuration_improvement(
        &self,
        env: Env,
        stage: &mut ConfigurationImprovementEvaluation,
    ) -> Result<ConfigurationImprovementDiagnosis> {
        self.api
            .diagnose_configuration_improvement(take(&mut stage.inner)?)
            .map(|inner| ConfigurationImprovementDiagnosis { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Freeze the diagnosed candidate for explicit effect-free review.
    ///
    /// # Errors
    ///
    /// Returns consumed-handle misuse or the API-owned rejection diagnostic.
    /// The input stage is consumed on both success and API rejection.
    #[napi]
    pub fn preview_configuration_improvement(
        &self,
        env: Env,
        stage: &mut ConfigurationImprovementDiagnosis,
    ) -> Result<ConfigurationImprovementPreview> {
        self.api
            .preview_configuration_improvement(take(&mut stage.inner)?)
            .map(|inner| ConfigurationImprovementPreview { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Prepare from an already admitted candidate while borrowing explanation.
    ///
    /// # Errors
    ///
    /// Returns consumed-handle misuse or the exact API preparation diagnostic.
    /// Every preparation failure leaves the explanation available for retry.
    #[napi]
    pub fn prepare_configuration_improvement_preview(
        &self,
        env: Env,
        explanation: &ConfigurationImprovementExplanation,
        candidate: &PortableWorkspaceCompilation,
        request: ConfigurationImprovementCandidateRequest,
    ) -> Result<PreparedConfigurationImprovementPreview> {
        self.api
            .prepare_configuration_improvement_preview(
                borrow(&explanation.inner)?,
                &candidate.inner,
                request.into_api(),
            )
            .map(|inner| PreparedConfigurationImprovementPreview { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Consume a prepared preview and the exact explanation that prepared it.
    ///
    /// # Errors
    ///
    /// Consumed host arguments are rejected before either remaining handle is
    /// taken. The API rejects equal-coordinate but foreign explanation owners;
    /// once called, both inputs are consumed even when that check fails.
    #[napi]
    pub fn commit_configuration_improvement_preview(
        &self,
        env: Env,
        explanation: &mut ConfigurationImprovementExplanation,
        prepared: &mut PreparedConfigurationImprovementPreview,
    ) -> Result<ConfigurationImprovementPreview> {
        borrow(&explanation.inner)?;
        borrow(&prepared.inner)?;
        self.api
            .commit_configuration_improvement_preview(
                take(&mut explanation.inner)?,
                take(&mut prepared.inner)?,
            )
            .map(|inner| ConfigurationImprovementPreview { inner: Some(inner) })
            .map_err(|error| diagnostic_error(&env, error))
    }
}

#[napi]
impl ConfigurationImprovementInspection {
    /// Admitted principal workspace.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn workspace_id(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.workspace_id().to_owned())
    }
    /// Exact baseline admitted-universe digest.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.principal_universe_digest().to_owned())
    }
}

#[napi]
impl ConfigurationImprovementExplanation {
    /// Exact baseline profile digest.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn profile_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.profile_digest().to_owned())
    }
    /// Exact private instance digest, not native-readiness proof.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn instance_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.instance_digest().to_owned())
    }
}

#[napi]
impl ConfigurationImprovementEvaluation {
    /// Exact candidate principal universe digest from admitted candidate compilation.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn candidate_principal_universe_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?
            .candidate_principal_universe_digest()
            .to_owned())
    }
    /// Exact candidate profile digest from admitted candidate compilation.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn candidate_profile_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.candidate_profile_digest().to_owned())
    }
    /// Exact candidate instance digest from admitted candidate compilation.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn candidate_instance_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.candidate_instance_digest().to_owned())
    }
}

#[napi]
impl ConfigurationImprovementDiagnosis {
    /// Exact canonical derivation receipt identity.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi(getter)]
    pub fn derivation_receipt_digest(&self) -> Result<String> {
        Ok(borrow(&self.inner)?.derivation_receipt_digest().to_owned())
    }
}

#[napi]
impl PreparedConfigurationImprovementPreview {
    /// Snapshot the bounded API-owned summary before exact-owner commit.
    ///
    /// # Errors
    ///
    /// Returns a host error after consumption.
    #[napi]
    pub fn summary(&self) -> Result<ConfigurationImprovementPreviewSummary> {
        Ok(ConfigurationImprovementPreviewSummary {
            inner: borrow(&self.inner)?.summary().clone(),
        })
    }
}

#[napi]
impl ConfigurationImprovementPreview {
    /// Snapshot exact review coordinates without source or native values.
    ///
    /// # Errors
    ///
    /// Returns a host error after terminal consumption.
    #[napi]
    pub fn summary(&self) -> Result<ConfigurationImprovementPreviewSummary> {
        Ok(ConfigurationImprovementPreviewSummary {
            inner: borrow(&self.inner)?.summary().clone(),
        })
    }
    /// Project the retained comparison identity and cardinalities.
    ///
    /// # Errors
    ///
    /// Returns a host error after terminal consumption.
    #[napi]
    pub fn comparison_summary(&self) -> Result<WorkspaceComparisonSummary> {
        Ok(WorkspaceComparisonSummary::from_api(
            borrow(&self.inner)?.comparison().summary(),
        ))
    }
    /// Page the existing change-root owner without deriving another graph.
    ///
    /// # Errors
    ///
    /// Returns a host error after terminal consumption.
    #[napi]
    pub fn change_roots(&self, offset: u32, limit: u16) -> Result<WorkspaceChangeRootPage> {
        Ok(WorkspaceChangeRootPage::from_api(
            borrow(&self.inner)?
                .comparison()
                .change_roots(offset, limit),
        ))
    }
    /// Page the canonical complete impact partition.
    ///
    /// # Errors
    ///
    /// Returns a host error after terminal consumption.
    #[napi]
    pub fn impacts(&self, offset: u32, limit: u16) -> Result<WorkspaceImpactPage> {
        Ok(WorkspaceImpactPage::from_api(
            borrow(&self.inner)?.comparison().impacts(offset, limit),
        ))
    }
    /// Explain a subject through the same retained comparison owner.
    ///
    /// # Errors
    ///
    /// Returns consumed-handle misuse or the exact API subject diagnostic.
    #[napi]
    pub fn explain(&self, env: Env, subject: &WorkspaceSubject) -> Result<WorkspaceExplanation> {
        borrow(&self.inner)?
            .comparison()
            .explain(&subject.inner)
            .map(WorkspaceExplanation::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }
    /// Inspect exact candidate source and proof evidence without recompiling.
    ///
    /// The preview retains the candidate compilation that owns this query. Resume
    /// with the same preview, subject, raw limits, and opaque continuation. The
    /// returned rows grant no source-disclosure or configuration-write authority.
    ///
    /// # Errors
    ///
    /// Invalid limits produce a host-argument error. A consumed preview, a cursor
    /// from another owner, or inconsistent retained proof returns its existing error.
    #[napi]
    pub fn inspect_candidate_subject(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
        input_limit: f64,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
    ) -> Result<PortableWorkspaceSubjectInspection> {
        borrow(&self.inner)?
            .inspect_candidate_subject(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(input_limit)?,
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
            )
            .map(|inner| PortableWorkspaceSubjectInspection { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
    /// Retain the immutable baseline profile for its existing query interface.
    ///
    /// # Errors
    ///
    /// Returns a host error after terminal consumption.
    #[napi]
    pub fn baseline_profile(&self) -> Result<EffectiveProfileCompilation> {
        Ok(EffectiveProfileCompilation {
            inner: borrow(&self.inner)?.baseline_profile().clone(),
        })
    }
    /// Retain the immutable candidate profile without recompiling its source.
    ///
    /// # Errors
    ///
    /// Returns a host error after terminal consumption.
    #[napi]
    pub fn candidate_profile(&self) -> Result<EffectiveProfileCompilation> {
        Ok(EffectiveProfileCompilation {
            inner: borrow(&self.inner)?.candidate_profile().clone(),
        })
    }
    /// Accept for presentation to a separate authority/effect gate.
    ///
    /// # Errors
    ///
    /// Returns a host error if already consumed. This transition causes no effect.
    #[napi]
    pub fn accept(&mut self) -> Result<ConfigurationImprovementTerminal> {
        Ok(ConfigurationImprovementTerminal {
            inner: take(&mut self.inner)?.accept(),
        })
    }
    /// Reject the candidate without changing any source.
    ///
    /// # Errors
    ///
    /// Returns a host error if already consumed. This transition causes no effect.
    #[napi]
    pub fn reject(&mut self) -> Result<ConfigurationImprovementTerminal> {
        Ok(ConfigurationImprovementTerminal {
            inner: take(&mut self.inner)?.reject(),
        })
    }
    /// Cancel review without asserting a native cancellation.
    ///
    /// # Errors
    ///
    /// Returns a host error if already consumed. This transition causes no effect.
    #[napi]
    pub fn cancel(&mut self) -> Result<ConfigurationImprovementTerminal> {
        Ok(ConfigurationImprovementTerminal {
            inner: take(&mut self.inner)?.cancel(),
        })
    }
    /// Mark stale after the caller proves its separately owned coordinate changed.
    ///
    /// # Errors
    ///
    /// Returns a host error if already consumed. This transition causes no effect.
    #[napi]
    pub fn mark_stale(&mut self) -> Result<ConfigurationImprovementTerminal> {
        Ok(ConfigurationImprovementTerminal {
            inner: take(&mut self.inner)?.mark_stale(),
        })
    }
}

/// Immutable terminal evidence, not mutation authority or native settlement.
#[napi]
pub struct ConfigurationImprovementTerminal {
    inner: api::ConfigurationImprovementTerminal,
}
#[napi]
impl ConfigurationImprovementTerminal {
    /// Explicit caller-selected disposition.
    #[napi(getter)]
    pub fn disposition(&self) -> ConfigurationImprovementDisposition {
        self.inner.disposition().into()
    }
    /// Exact summary of the preview governed by this disposition.
    #[napi]
    pub fn preview(&self) -> ConfigurationImprovementPreviewSummary {
        ConfigurationImprovementPreviewSummary {
            inner: self.inner.preview().clone(),
        }
    }
}

/// Closed digest transition; payload fields follow this discriminant.
#[napi]
pub enum ConfigurationImprovementCoordinateChangeKind {
    /// Both compilations share the exact digest.
    Unchanged,
    /// Candidate and baseline have different digests.
    Changed,
}

/// Lossless digest transition; unselected variant fields are absent.
#[napi(object, object_from_js = false)]
pub struct ConfigurationImprovementCoordinateChange {
    /// Exact API-owned variant.
    pub kind: ConfigurationImprovementCoordinateChangeKind,
    /// Shared digest, present only for unchanged.
    pub digest: Option<String>,
    /// Prior digest, present only for changed.
    pub prior_digest: Option<String>,
    /// Candidate digest, present only for changed.
    pub candidate_digest: Option<String>,
}
impl ConfigurationImprovementCoordinateChange {
    fn from_api(value: &api::ConfigurationImprovementCoordinateChange) -> Self {
        match value {
            api::ConfigurationImprovementCoordinateChange::Unchanged { digest } => Self {
                kind: ConfigurationImprovementCoordinateChangeKind::Unchanged,
                digest: Some(digest.clone()),
                prior_digest: None,
                candidate_digest: None,
            },
            api::ConfigurationImprovementCoordinateChange::Changed {
                prior_digest,
                candidate_digest,
            } => Self {
                kind: ConfigurationImprovementCoordinateChangeKind::Changed,
                digest: None,
                prior_digest: Some(prior_digest.clone()),
                candidate_digest: Some(candidate_digest.clone()),
            },
        }
    }
}

/// Complete owned summary with exact API bytes and separately typed coordinates.
///
/// Counts follow [`api::PortableWorkspaceComparison`]'s complete disjoint
/// impact partition. This value carries no source bytes or effect authority.
#[napi]
pub struct ConfigurationImprovementPreviewSummary {
    inner: api::ConfigurationImprovementPreviewSummary,
}
#[napi]
impl ConfigurationImprovementPreviewSummary {
    /// Principal workspace being reviewed.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }
    /// Selected project.
    #[napi(getter)]
    pub fn project_id(&self) -> String {
        self.inner.project_id().to_owned()
    }
    /// Selected execution workspace.
    #[napi(getter)]
    pub fn execution_workspace_id(&self) -> String {
        self.inner.execution_workspace_id().to_owned()
    }
    /// Baseline admitted-universe identity.
    #[napi(getter)]
    pub fn baseline_principal_universe_digest(&self) -> String {
        self.inner.baseline_principal_universe_digest().to_owned()
    }
    /// Baseline workspace semantic-IR identity.
    #[napi(getter)]
    pub fn baseline_workspace_semantic_ir_digest(&self) -> String {
        self.inner
            .baseline_workspace_semantic_ir_digest()
            .to_owned()
    }
    /// Baseline admission-evidence frontier.
    #[napi(getter)]
    pub fn baseline_workspace_admission_evidence_digest(&self) -> String {
        self.inner
            .baseline_workspace_admission_evidence_digest()
            .to_owned()
    }
    /// Baseline selected topology identity.
    #[napi(getter)]
    pub fn selected_topology_digest(&self) -> String {
        self.inner.selected_topology_digest().to_owned()
    }
    /// Baseline effective-profile identity.
    #[napi(getter)]
    pub fn profile_digest(&self) -> String {
        self.inner.profile_digest().to_owned()
    }
    /// Baseline private instance identity, not readiness.
    #[napi(getter)]
    pub fn instance_digest(&self) -> String {
        self.inner.instance_digest().to_owned()
    }
    /// Baseline admitted Package semantic identity.
    #[napi(getter)]
    pub fn package_semantic_ir_digest(&self) -> String {
        self.inner.package_semantic_ir_digest().to_owned()
    }
    /// Baseline composition-proof identity.
    #[napi(getter)]
    pub fn composition_plan_digest(&self) -> String {
        self.inner.composition_plan_digest().to_owned()
    }
    /// Candidate admitted-universe identity.
    #[napi(getter)]
    pub fn candidate_principal_universe_digest(&self) -> String {
        self.inner.candidate_principal_universe_digest().to_owned()
    }
    /// Candidate workspace semantic-IR identity.
    #[napi(getter)]
    pub fn candidate_workspace_semantic_ir_digest(&self) -> String {
        self.inner
            .candidate_workspace_semantic_ir_digest()
            .to_owned()
    }
    /// Candidate admission-evidence frontier.
    #[napi(getter)]
    pub fn candidate_workspace_admission_evidence_digest(&self) -> String {
        self.inner
            .candidate_workspace_admission_evidence_digest()
            .to_owned()
    }
    /// Candidate selected topology identity.
    #[napi(getter)]
    pub fn candidate_selected_topology_digest(&self) -> String {
        self.inner.candidate_selected_topology_digest().to_owned()
    }
    /// Candidate effective-profile identity.
    #[napi(getter)]
    pub fn candidate_profile_digest(&self) -> String {
        self.inner.candidate_profile_digest().to_owned()
    }
    /// Candidate private instance identity, not readiness.
    #[napi(getter)]
    pub fn candidate_instance_digest(&self) -> String {
        self.inner.candidate_instance_digest().to_owned()
    }
    /// Candidate admitted Package semantic identity.
    #[napi(getter)]
    pub fn candidate_package_semantic_ir_digest(&self) -> String {
        self.inner.candidate_package_semantic_ir_digest().to_owned()
    }
    /// Candidate composition-proof identity.
    #[napi(getter)]
    pub fn candidate_composition_plan_digest(&self) -> String {
        self.inner.candidate_composition_plan_digest().to_owned()
    }
    /// Exact derivation receipt identity.
    #[napi(getter)]
    pub fn derivation_receipt_digest(&self) -> String {
        self.inner.derivation_receipt_digest().to_owned()
    }
    /// Number of exact change roots.
    #[napi(getter)]
    pub fn change_root_count(&self) -> u32 {
        self.inner.change_root_count()
    }
    /// Number of affected subjects in the complete impact partition.
    #[napi(getter)]
    pub fn affected_count(&self) -> u32 {
        self.inner.affected_count()
    }
    /// Number of unaffected subjects in the complete impact partition.
    #[napi(getter)]
    pub fn unaffected_count(&self) -> u32 {
        self.inner.unaffected_count()
    }
    /// Selected topology transition.
    #[napi(getter)]
    pub fn selected_topology_change(&self) -> ConfigurationImprovementCoordinateChange {
        ConfigurationImprovementCoordinateChange::from_api(self.inner.selected_topology_change())
    }
    /// Portable profile transition.
    #[napi(getter)]
    pub fn profile_change(&self) -> ConfigurationImprovementCoordinateChange {
        ConfigurationImprovementCoordinateChange::from_api(self.inner.profile_change())
    }
    /// Private instance transition, independent of native realization.
    #[napi(getter)]
    pub fn instance_change(&self) -> ConfigurationImprovementCoordinateChange {
        ConfigurationImprovementCoordinateChange::from_api(self.inner.instance_change())
    }
    /// Admitted Package semantic transition.
    #[napi(getter)]
    pub fn package_semantic_ir_change(&self) -> ConfigurationImprovementCoordinateChange {
        ConfigurationImprovementCoordinateChange::from_api(self.inner.package_semantic_ir_change())
    }
    /// Composition-proof transition.
    #[napi(getter)]
    pub fn composition_plan_change(&self) -> ConfigurationImprovementCoordinateChange {
        ConfigurationImprovementCoordinateChange::from_api(self.inner.composition_plan_change())
    }
    /// Exact API-owned canonical bytes, not a Node serialization.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}
