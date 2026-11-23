//! Native-context PaleJS projections over API-owned one-shot use cases.

use std::sync::Arc;

use napi::bindgen_prelude::{AsyncTask, Buffer, ClassInstance};
use napi::{Env, Error, Result, Status, Task};
use napi_derive::napi;
use pale_sdk::{
    ApiError, CompilationDocument as ApiCompilationDocument, MachineLocalEffectCandidate,
    OneShotAdmittedPlan as ApiOneShotAdmittedPlan, OneShotBaselineEntry,
    OneShotBaselineObservation, OneShotBoundPlan as ApiOneShotBoundPlan,
    OneShotCompilation as ApiOneShotCompilation,
    OneShotCompilationRequest as ApiOneShotCompilationRequest,
    OneShotPlanCandidate as ApiOneShotPlanCandidate, OneShotPlanKind as ApiOneShotPlanKind,
    OneShotPlanningRequest as ApiOneShotPlanningRequest,
    OneShotPriorMaterialization as ApiOneShotPriorMaterialization,
    OneShotPrivateInputs as ApiOneShotPrivateInputs, PaleApi,
};

use crate::error::diagnostic_error;
use crate::{Pale, PortableWorkspaceRequest};

/// One labelled Package or lifecycle document captured in Node-owned memory.
#[napi(object, object_to_js = false, js_name = "CompilationDocument")]
pub struct CompilationDocument {
    /// Logical source label with no host-path authority.
    pub label: String,
    /// Complete captured document bytes.
    pub bytes: Buffer,
}

impl CompilationDocument {
    /// Consume the host wrapper into the API document owner.
    pub(crate) fn into_api(self) -> ApiCompilationDocument {
        ApiCompilationDocument::new(self.label, self.bytes.to_vec())
    }
}

/// Fixed-role private source batch for one complete compilation.
#[napi(object, object_to_js = false, js_name = "OneShotPrivateInputs")]
pub struct OneShotPrivateInputs {
    /// Labelled private capability-policy document.
    pub capabilities: CompilationDocument,
    /// Labelled selected machine-binding document.
    pub machine_binding: CompilationDocument,
    /// Labelled admitted distribution-lock document.
    pub distribution_lock: CompilationDocument,
    /// Labelled lifecycle-guidance documents.
    pub lifecycle_guidance: Vec<CompilationDocument>,
}

impl OneShotPrivateInputs {
    /// Consume host memory into the API-owned fixed-role request.
    pub(crate) fn into_api(self) -> ApiOneShotPrivateInputs {
        ApiOneShotPrivateInputs::new(
            self.capabilities.into_api(),
            self.machine_binding.into_api(),
            self.distribution_lock.into_api(),
            self.lifecycle_guidance
                .into_iter()
                .map(CompilationDocument::into_api)
                .collect(),
        )
    }
}

/// Complete captured source family for one effect-free one-shot compilation.
#[napi(object, object_to_js = false, js_name = "OneShotCompilationRequest")]
pub struct OneShotCompilationRequest {
    /// Canonical project selector candidate.
    pub project_id: String,
    /// Canonical execution-workspace selector candidate.
    pub execution_workspace_id: String,
    /// Canonical machine selector candidate.
    pub machine_id: String,
    /// Fixed-role private source batch.
    pub private_inputs: OneShotPrivateInputs,
    /// Complete captured Package document batch.
    pub package_documents: Vec<CompilationDocument>,
    /// Complete captured workspace source batch.
    pub workspace: PortableWorkspaceRequest,
}

impl OneShotCompilationRequest {
    /// Consume every captured source into the API request owner.
    fn into_api(self) -> ApiOneShotCompilationRequest {
        ApiOneShotCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
            self.package_documents
                .into_iter()
                .map(CompilationDocument::into_api)
                .collect(),
            self.workspace.into_api(),
        )
    }
}

/// Opaque admitted one-shot compilation; garbage collection has no effect.
#[napi(js_name = "OneShotCompilation")]
pub struct OneShotCompilation {
    inner: ApiOneShotCompilation,
}

/// Owned one-shot computation scheduled on the Node-API worker pool.
///
/// Only Rust-owned input, façade and output cross the worker boundary. The API
/// result remains intact until JavaScript-thread resolution; this task grants
/// no cancellation or native-effect authority.
pub struct OneShotCompilationTask {
    api: Arc<PaleApi>,
    request: Option<ApiOneShotCompilationRequest>,
}

#[napi]
impl Task for OneShotCompilationTask {
    type Output = std::result::Result<ApiOneShotCompilation, ApiError>;
    type JsValue = OneShotCompilation;

    fn compute(&mut self) -> Result<Self::Output> {
        let request = self.request.take().ok_or_else(|| {
            Error::new(
                Status::GenericFailure,
                "one-shot task input was already consumed",
            )
        })?;
        Ok(self.api.compile_one_shot(request))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output
            .map(|inner| OneShotCompilation { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}

/// Flat binding-safe summary of one admitted one-shot compilation.
#[napi(object, object_from_js = false, js_name = "OneShotSummary")]
pub struct OneShotSummary {
    /// Complete API-owned canonical summary bytes.
    pub canonical_bytes: Buffer,
    /// Canonical project identity.
    pub project_id: String,
    /// Canonical execution-workspace identity.
    pub execution_workspace_id: String,
    /// Effective profile digest.
    pub profile_digest: String,
    /// Complete private-instance digest.
    pub instance_digest: String,
    /// Post-render one-shot output digest.
    pub one_shot_compilation_digest: String,
    /// Descriptor-manifest digest.
    pub schema_manifest_sha256: String,
    /// Raw-schema-set digest.
    pub schema_set_sha256: String,
    /// Number of selected repositories.
    pub repository_count: u32,
    /// Stable repository identities.
    pub repository_ids: Vec<String>,
}

impl OneShotSummary {
    /// Project only API-owned scalars; no application type crosses the leaf.
    pub(crate) fn from_api(summary: pale_sdk::OneShotCompilationSummary) -> Self {
        Self {
            canonical_bytes: summary.canonical_bytes().to_vec().into(),
            project_id: summary.project_id().to_owned(),
            execution_workspace_id: summary.execution_workspace_id().to_owned(),
            profile_digest: summary.profile_digest().to_owned(),
            instance_digest: summary.instance_digest().to_owned(),
            one_shot_compilation_digest: summary.one_shot_compilation_digest().to_owned(),
            schema_manifest_sha256: summary.schema_manifest_sha256().to_owned(),
            schema_set_sha256: summary.schema_set_sha256().to_owned(),
            repository_count: summary.repository_count(),
            repository_ids: summary
                .repositories()
                .iter()
                .map(|repository| repository.repository_id().to_owned())
                .collect(),
        }
    }
}

/// Closed effect-free repository-plan operation.
#[napi]
#[derive(Clone, Copy)]
pub enum OneShotPlanKind {
    /// Reconcile desired surfaces with prior ownership.
    Apply,
    /// Explicitly adopt matching pre-existing targets.
    Adopt,
    /// Remove only previously owned surfaces.
    Remove,
}

impl From<ApiOneShotPlanKind> for OneShotPlanKind {
    fn from(value: ApiOneShotPlanKind) -> Self {
        match value {
            ApiOneShotPlanKind::Apply => Self::Apply,
            ApiOneShotPlanKind::Adopt => Self::Adopt,
            ApiOneShotPlanKind::Remove => Self::Remove,
        }
    }
}

impl From<OneShotPlanKind> for ApiOneShotPlanKind {
    fn from(value: OneShotPlanKind) -> Self {
        match value {
            OneShotPlanKind::Apply => Self::Apply,
            OneShotPlanKind::Adopt => Self::Adopt,
            OneShotPlanKind::Remove => Self::Remove,
        }
    }
}

/// Prior Pale ownership-state claim for pure planning.
#[napi(js_name = "OneShotPriorMaterialization")]
pub struct OneShotPriorMaterialization {
    state: NodePriorState,
}

#[derive(Clone)]
enum NodePriorState {
    Unowned,
    Owned(Vec<u8>),
}

#[napi]
impl OneShotPriorMaterialization {
    /// Construct an explicit unowned prior-state claim.
    #[napi(factory)]
    pub fn unowned() -> Self {
        Self {
            state: NodePriorState::Unowned,
        }
    }

    /// Construct an explicit owned prior-state claim.
    #[napi(factory)]
    pub fn owned(canonical_state: Buffer) -> Self {
        Self {
            state: NodePriorState::Owned(canonical_state.to_vec()),
        }
    }

    /// Rebuild the API enum without interpreting the claimed bytes.
    fn to_api(&self) -> ApiOneShotPriorMaterialization {
        match &self.state {
            NodePriorState::Owned(canonical_state) => ApiOneShotPriorMaterialization::Owned {
                canonical_state: canonical_state.clone(),
            },
            NodePriorState::Unowned => ApiOneShotPriorMaterialization::Unowned,
        }
    }
}

/// One caller-captured pre-mutation baseline observation.
#[napi(js_name = "BaselineEntry")]
pub struct BaselineEntry {
    relative_path: String,
    state: NodeBaselineState,
}

#[derive(Clone)]
enum NodeBaselineState {
    Absent,
    Present { sha256: String, unix_mode: u32 },
}

#[napi]
impl BaselineEntry {
    /// Construct an explicit absent-file observation.
    #[napi(factory)]
    pub fn absent(relative_path: String) -> Self {
        Self {
            relative_path,
            state: NodeBaselineState::Absent,
        }
    }

    /// Construct an explicit present-file observation.
    #[napi(factory)]
    pub fn present(relative_path: String, sha256: String, unix_mode: u32) -> Self {
        Self {
            relative_path,
            state: NodeBaselineState::Present { sha256, unix_mode },
        }
    }

    /// Consume the host claim without upgrading it to native evidence.
    fn to_api(&self) -> OneShotBaselineEntry {
        let observation = match &self.state {
            NodeBaselineState::Absent => OneShotBaselineObservation::Absent,
            NodeBaselineState::Present { sha256, unix_mode } => {
                OneShotBaselineObservation::Present {
                    sha256: sha256.clone(),
                    unix_mode: *unix_mode,
                }
            }
        };
        OneShotBaselineEntry::new(self.relative_path.clone(), observation)
    }
}

/// Complete pure-planning request.
#[napi(js_name = "OneShotPlanningRequest")]
pub struct OneShotPlanningRequest {
    repository_id: String,
    kind: ApiOneShotPlanKind,
    prior: NodePriorState,
    baseline: Vec<OneShotBaselineEntry>,
}

/// Complete effect-free observation-target request.
#[napi(js_name = "OneShotObservationRequest")]
pub struct OneShotObservationRequest {
    repository_id: String,
    kind: ApiOneShotPlanKind,
    prior: NodePriorState,
}

#[napi]
impl OneShotObservationRequest {
    /// Capture one cohesive observation-target request.
    #[napi(constructor)]
    pub fn new(
        repository_id: String,
        kind: OneShotPlanKind,
        prior: &OneShotPriorMaterialization,
    ) -> Self {
        Self {
            repository_id,
            kind: kind.into(),
            prior: prior.state.clone(),
        }
    }
}

#[napi]
impl OneShotPlanningRequest {
    /// Capture one cohesive planning request from explicit closed observations.
    #[napi(constructor)]
    pub fn new(
        repository_id: String,
        kind: OneShotPlanKind,
        prior: &OneShotPriorMaterialization,
        baseline: Vec<ClassInstance<BaselineEntry>>,
    ) -> Self {
        Self {
            repository_id,
            kind: kind.into(),
            prior: prior.state.clone(),
            baseline: baseline.iter().map(|entry| entry.to_api()).collect(),
        }
    }

    /// Consume the host request into the API planning owner.
    fn to_api(&self) -> ApiOneShotPlanningRequest {
        ApiOneShotPlanningRequest::new(
            self.repository_id.clone(),
            self.kind,
            OneShotPriorMaterialization {
                state: self.prior.clone(),
            }
            .to_api(),
            self.baseline.clone(),
        )
    }
}

/// Canonical effect-free plan candidate and its flat transition counts.
///
/// The private API value is linear: binding machine effects consumes it
/// exactly once. The copied identity fields remain inspectable so JavaScript
/// garbage collection or a repeated host call cannot recreate planning
/// authority.
#[napi(js_name = "OneShotPlanCandidate")]
pub struct OneShotPlanCandidate {
    inner: Option<ApiOneShotPlanCandidate>,
    kind: ApiOneShotPlanKind,
    repository_id: String,
    plan_digest: String,
    one_shot_compilation_digest: String,
    operation_count: u32,
    deferred_effect_count: u32,
    canonical_bytes: Vec<u8>,
}

#[napi]
impl OneShotPlanCandidate {
    /// Requested plan operation.
    #[napi(getter)]
    pub fn kind(&self) -> OneShotPlanKind {
        self.kind.into()
    }

    /// Selected logical repository.
    #[napi(getter)]
    pub fn repository_id(&self) -> String {
        self.repository_id.clone()
    }

    /// Canonical complete-plan digest.
    #[napi(getter)]
    pub fn plan_digest(&self) -> String {
        self.plan_digest.clone()
    }

    /// One-shot output identity used by planning.
    #[napi(getter)]
    pub fn one_shot_compilation_digest(&self) -> String {
        self.one_shot_compilation_digest.clone()
    }

    /// Planned operation count.
    #[napi(getter)]
    pub fn operation_count(&self) -> u32 {
        self.operation_count
    }

    /// Deferred host-effect count.
    #[napi(getter)]
    pub fn deferred_effect_count(&self) -> u32 {
        self.deferred_effect_count
    }

    /// Copy canonical `agent-surface-plan.v1` bytes.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.canonical_bytes.clone().into()
    }

    fn from_api(inner: ApiOneShotPlanCandidate) -> Self {
        Self {
            kind: inner.kind(),
            repository_id: inner.repository_id().to_owned(),
            plan_digest: inner.plan_digest().to_owned(),
            one_shot_compilation_digest: inner.one_shot_compilation_digest().to_owned(),
            operation_count: inner.operation_count(),
            deferred_effect_count: inner.deferred_effect_count(),
            canonical_bytes: inner.canonical_bytes().to_vec(),
            inner: Some(inner),
        }
    }

    fn take_api(&mut self) -> Result<ApiOneShotPlanCandidate> {
        self.inner.take().ok_or_else(|| {
            Error::new(
                Status::InvalidArg,
                "one-shot plan candidate has already been consumed",
            )
        })
    }
}

/// Complete deterministic plan after machine-local observations are bound.
#[napi(js_name = "OneShotBoundPlan")]
pub struct OneShotBoundPlan {
    inner: Option<ApiOneShotBoundPlan>,
    kind: ApiOneShotPlanKind,
    repository_id: String,
    canonical_bytes: Vec<u8>,
}

#[napi]
impl OneShotBoundPlan {
    /// Requested plan operation.
    #[napi(getter)]
    pub fn kind(&self) -> OneShotPlanKind {
        self.kind.into()
    }

    /// Selected logical repository.
    #[napi(getter)]
    pub fn repository_id(&self) -> String {
        self.repository_id.clone()
    }

    /// Copy canonical complete materialization-plan bytes.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.canonical_bytes.clone().into()
    }

    fn from_api(inner: ApiOneShotBoundPlan) -> Self {
        Self {
            kind: inner.kind(),
            repository_id: inner.repository_id().to_owned(),
            canonical_bytes: inner.canonical_bytes().to_vec(),
            inner: Some(inner),
        }
    }

    fn take_api(&mut self) -> Result<ApiOneShotBoundPlan> {
        self.inner.take().ok_or_else(|| {
            Error::new(
                Status::InvalidArg,
                "one-shot bound plan has already been consumed",
            )
        })
    }
}

/// Saved-plan equality proof with no native effect.
#[napi(object, object_from_js = false, js_name = "OneShotAdmittedPlan")]
pub struct OneShotAdmittedPlan {
    /// Admitted operation class.
    pub kind: OneShotPlanKind,
    /// Selected logical repository identity.
    pub repository_id: String,
    /// Digest of the admitted complete materialization plan.
    pub plan_digest: String,
}

impl OneShotAdmittedPlan {
    fn from_api(value: ApiOneShotAdmittedPlan) -> Self {
        Self {
            kind: value.kind().into(),
            repository_id: value.repository_id().to_owned(),
            plan_digest: value.plan_digest().to_owned(),
        }
    }
}

#[napi]
impl Pale {
    /// Compile captured sources on the Node-API worker pool, returning a promise.
    ///
    /// All source buffers are copied into owned API inputs before this call
    /// returns. Mutating or collecting the caller's buffers or façade cannot
    /// change the queued compilation. Admission and rendering run off the
    /// JavaScript thread; only result/diagnostic projection returns to it.
    ///
    /// # Errors
    ///
    /// Rejects the promise with the canonical API diagnostic when captured
    /// source or context fails admission. Host argument decoding or queue
    /// submission may fail before work starts. No abort mechanism is exposed.
    #[napi]
    pub fn compile_one_shot(
        &self,
        request: OneShotCompilationRequest,
    ) -> AsyncTask<OneShotCompilationTask> {
        AsyncTask::new(OneShotCompilationTask {
            api: Arc::clone(&self.api),
            request: Some(request.into_api()),
        })
    }

    /// Return the flat binding-safe summary of one admitted compilation.
    #[napi]
    pub fn one_shot_summary(&self, compilation: &OneShotCompilation) -> OneShotSummary {
        OneShotSummary::from_api(compilation.inner.summary())
    }

    /// Derive the exact target set a host must observe before pure planning.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the
    /// repository, plan kind, or claimed prior state is invalid.
    #[napi]
    pub fn one_shot_observation_targets(
        &self,
        env: Env,
        compilation: &OneShotCompilation,
        request: ClassInstance<OneShotObservationRequest>,
    ) -> Result<Vec<String>> {
        self.api
            .one_shot_observation_targets(
                &compilation.inner,
                request.repository_id.clone(),
                request.kind,
                &OneShotPriorMaterialization {
                    state: request.prior.clone(),
                }
                .to_api(),
            )
            .map(|targets| {
                targets
                    .into_iter()
                    .map(|target| target.relative_path().to_owned())
                    .collect()
            })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Construct one deterministic plan candidate without host effects.
    ///
    /// # Errors
    ///
    /// Returns a host-shape error for an incomplete baseline tuple, or a native
    /// error with separate `diagnostic` bytes when planning rejects the claim.
    #[napi]
    pub fn plan_one_shot(
        &self,
        env: Env,
        compilation: &OneShotCompilation,
        request: ClassInstance<OneShotPlanningRequest>,
    ) -> Result<OneShotPlanCandidate> {
        let request = request.to_api();
        self.api
            .plan_one_shot(&compilation.inner, request)
            .map(OneShotPlanCandidate::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Bind canonical machine-local effect candidates into one complete plan.
    ///
    /// The API consumes `candidate` once and owns validation and deterministic
    /// ordering. This method performs no host observation or native effect.
    ///
    /// # Errors
    ///
    /// Returns a host-shape error if the candidate was already consumed, or a
    /// native error with separate `diagnostic` bytes when API admission rejects
    /// a supplied machine-local effect.
    #[napi]
    pub fn bind_one_shot_machine_effects(
        &self,
        env: Env,
        candidate: &mut OneShotPlanCandidate,
        effects: Vec<Buffer>,
    ) -> Result<OneShotBoundPlan> {
        let candidate = candidate.take_api()?;
        self.api
            .bind_one_shot_machine_effects(
                candidate,
                effects
                    .into_iter()
                    .map(|bytes| MachineLocalEffectCandidate::new(bytes.to_vec()))
                    .collect(),
            )
            .map(OneShotBoundPlan::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Admit saved bytes against one freshly rebuilt bound plan.
    ///
    /// Exact equality and plan identity remain API-owned. Success creates an
    /// admission proof only; it performs no filesystem mutation.
    ///
    /// # Errors
    ///
    /// Returns a host-shape error if the bound plan was already consumed, or a
    /// native error with separate `diagnostic` bytes when saved-plan admission
    /// rejects the supplied bytes.
    #[napi]
    pub fn admit_one_shot_saved_plan(
        &self,
        env: Env,
        rebuilt: &mut OneShotBoundPlan,
        saved_bytes: Buffer,
    ) -> Result<OneShotAdmittedPlan> {
        let rebuilt = rebuilt.take_api()?;
        self.api
            .admit_one_shot_saved_plan(rebuilt, &saved_bytes)
            .map(OneShotAdmittedPlan::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }
}
