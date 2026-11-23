//! Native-local PaleJS projections over API-owned host composition.
//!
//! This module translates owned JavaScript values into the `native-local`
//! [`pale_sdk`] façade. Filesystem/process composition, planning, saved-plan
//! admission, settlement, reconciliation, and diagnostic policy stay behind
//! that façade; this leaf neither imports adapters nor recreates their laws.

use napi::bindgen_prelude::Buffer;
use napi::{Env, Result};
use napi_derive::napi;
use pale_sdk::{
    CompiledInterfaceIdentity as ApiCompiledInterfaceIdentity,
    DurablePublicationMechanism as ApiDurablePublicationMechanism,
    LocalMaterializationAction as ApiLocalMaterializationAction,
    LocalMaterializationAdmissionRequest as ApiLocalMaterializationAdmissionRequest,
    LocalMaterializationAdmittedRun as ApiLocalMaterializationAdmittedRun,
    LocalMaterializationDrift as ApiLocalMaterializationDrift,
    LocalMaterializationMachineEffectKind as ApiLocalMaterializationMachineEffectKind,
    LocalMaterializationOwnershipChange as ApiLocalMaterializationOwnershipChange,
    LocalMaterializationOwnershipObservation as ApiLocalMaterializationOwnershipObservation,
    LocalMaterializationReceiptObservation as ApiLocalMaterializationReceiptObservation,
    LocalMaterializationReconciliation as ApiLocalMaterializationReconciliation,
    LocalMaterializationRunOutcome as ApiLocalMaterializationRunOutcome,
    LocalMaterializationRunResult as ApiLocalMaterializationRunResult,
    LocalMaterializationTargetAdmission as ApiLocalMaterializationTargetAdmission,
    LocalMaterializationTargetOutcome as ApiLocalMaterializationTargetOutcome,
    LocalMaterializationTargetResult as ApiLocalMaterializationTargetResult,
    LocalNativeActionInputRequest as ApiLocalNativeActionInputRequest,
    LocalOneShotCompilation as ApiLocalOneShotCompilation,
    LocalOneShotCompilationRequest as ApiLocalOneShotCompilationRequest,
    LocalOneShotPlan as ApiLocalOneShotPlan, LocalOneShotPlanCounts as ApiLocalOneShotPlanCounts,
    LocalOneShotPlanRequest as ApiLocalOneShotPlanRequest,
    LocalPackageDistributionRequest as ApiLocalPackageDistributionRequest,
    LocalRepositoryStatus as ApiLocalRepositoryStatus, LocalWorkspaceLocator, NativeActionRequest,
};

use crate::error::diagnostic_error;
use crate::{OneShotPlanKind, OneShotSummary, PackageDistributionInspection, Pale};

/// Local source selection for one complete one-shot compilation.
#[napi(
    object,
    object_to_js = false,
    js_name = "LocalOneShotCompilationRequest"
)]
pub struct LocalOneShotCompilationRequest {
    /// Absolute UTF-8 spelling of `pale-workspace.toml`.
    pub workspace_manifest: String,
    /// Canonical project selector candidate.
    pub project_id: String,
    /// Canonical execution-workspace selector candidate.
    pub execution_workspace_id: String,
    /// Canonical machine selector candidate.
    pub machine_id: String,
}

/// Local source selection for Package distribution inspection.
#[napi(
    object,
    object_to_js = false,
    js_name = "LocalPackageDistributionRequest"
)]
pub struct LocalPackageDistributionRequest {
    /// Absolute UTF-8 spelling of `pale-workspace.toml`.
    pub workspace_manifest: String,
    /// Canonical project selector candidate.
    pub project_id: String,
    /// Canonical execution-workspace selector candidate.
    pub execution_workspace_id: String,
    /// Canonical machine selector candidate.
    pub machine_id: String,
}

/// Confined local source for one effect-free native-action request.
#[napi(
    object,
    object_to_js = false,
    js_name = "LocalNativeActionInputRequest"
)]
pub struct LocalNativeActionInputRequest {
    /// Exact compiled action identity.
    pub action_id: String,
    /// Absolute retained-directory spelling.
    pub root: String,
    /// Confined path relative to `root`.
    pub relative_path: String,
}

/// Opaque path-free native-action request captured by the API owner.
#[napi(js_name = "CapturedNativeActionRequest")]
pub struct CapturedNativeActionRequest {
    inner: NativeActionRequest,
}

/// Opaque local compilation retaining API-private host capabilities.
///
/// The handle itself grants no plan admission, mutation, runtime activation,
/// or resident authority. JavaScript can pass it only to the explicit local
/// operations exposed by [`Pale`].
#[napi(js_name = "LocalOneShotCompilation")]
pub struct LocalOneShotCompilation {
    inner: ApiLocalOneShotCompilation,
}

#[napi]
impl LocalOneShotCompilation {
    /// Return the same bounded summary projected by captured one-shot compilation.
    #[napi]
    pub fn summary(&self) -> OneShotSummary {
        OneShotSummary::from_api(self.inner.summary())
    }
}

/// Closed repository or machine-local transition shown by local status.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationAction {
    /// Create a previously absent file.
    Create,
    /// Adopt matching pre-existing bytes under explicit adoption policy.
    Adopt,
    /// Replace an owned file whose desired bytes changed.
    Update,
    /// Delete a previously owned file.
    Delete,
    /// Replace only Pale's managed block while preserving other bytes.
    Splice,
}

impl From<ApiLocalMaterializationAction> for LocalMaterializationAction {
    fn from(value: ApiLocalMaterializationAction) -> Self {
        match value {
            ApiLocalMaterializationAction::Create => Self::Create,
            ApiLocalMaterializationAction::Adopt => Self::Adopt,
            ApiLocalMaterializationAction::Update => Self::Update,
            ApiLocalMaterializationAction::Delete => Self::Delete,
            ApiLocalMaterializationAction::Splice => Self::Splice,
        }
    }
}

/// Ownership-record transition required by local status.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationOwnershipChange {
    /// Establish the first trusted ownership state.
    Initialize,
    /// Advance an existing trusted ownership state.
    Advance,
}

impl From<ApiLocalMaterializationOwnershipChange> for LocalMaterializationOwnershipChange {
    fn from(value: ApiLocalMaterializationOwnershipChange) -> Self {
        match value {
            ApiLocalMaterializationOwnershipChange::Initialize => Self::Initialize,
            ApiLocalMaterializationOwnershipChange::Advance => Self::Advance,
        }
    }
}

/// Closed machine-local policy effect visible through local status.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationMachineEffectKind {
    /// Managed protection in the verified Git exclude file.
    GitExcludeProtection,
}

impl From<ApiLocalMaterializationMachineEffectKind> for LocalMaterializationMachineEffectKind {
    fn from(value: ApiLocalMaterializationMachineEffectKind) -> Self {
        match value {
            ApiLocalMaterializationMachineEffectKind::GitExcludeProtection => {
                Self::GitExcludeProtection
            }
        }
    }
}

/// Non-interchangeable local-drift variant.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationDriftKind {
    /// One generated repository surface differs.
    Surface,
    /// The trusted ownership record must change.
    Ownership,
    /// One machine-local policy file differs.
    MachineLocal,
}

/// One exact API-derived reason a repository differs from its projection.
#[napi(object, object_from_js = false, js_name = "LocalMaterializationDrift")]
pub struct LocalMaterializationDrift {
    /// Closed variant retaining the drift's semantic class.
    pub kind: LocalMaterializationDriftKind,
    /// Canonical presentation label owned by the API.
    pub status_label: String,
    /// Confined relative path when the variant identifies one file.
    pub relative_path: Option<String>,
    /// Required file transition for surface or machine-local drift.
    pub action: Option<LocalMaterializationAction>,
    /// Required ownership transition for ownership drift.
    pub ownership_change: Option<LocalMaterializationOwnershipChange>,
    /// Closed machine-local effect kind for machine-local drift.
    pub machine_effect_kind: Option<LocalMaterializationMachineEffectKind>,
}

impl LocalMaterializationDrift {
    fn from_api(value: &ApiLocalMaterializationDrift) -> Self {
        let status_label = value.status_label();
        match value {
            ApiLocalMaterializationDrift::Surface {
                relative_path,
                action,
            } => Self {
                kind: LocalMaterializationDriftKind::Surface,
                status_label,
                relative_path: Some(relative_path.clone()),
                action: Some((*action).into()),
                ownership_change: None,
                machine_effect_kind: None,
            },
            ApiLocalMaterializationDrift::Ownership { change } => Self {
                kind: LocalMaterializationDriftKind::Ownership,
                status_label,
                relative_path: None,
                action: None,
                ownership_change: Some((*change).into()),
                machine_effect_kind: None,
            },
            ApiLocalMaterializationDrift::MachineLocal {
                kind,
                relative_path,
                action,
            } => Self {
                kind: LocalMaterializationDriftKind::MachineLocal,
                status_label,
                relative_path: Some(relative_path.clone()),
                action: Some((*action).into()),
                ownership_change: None,
                machine_effect_kind: Some((*kind).into()),
            },
        }
    }
}

/// Complete read-only comparison for one local repository.
#[napi(object, object_from_js = false, js_name = "LocalRepositoryStatus")]
pub struct LocalRepositoryStatus {
    /// Logical repository selected from the admitted compilation.
    pub repository_id: String,
    /// Whether every observed surface and ownership fact matches.
    pub is_clean: bool,
    /// Deterministically ordered drift partition.
    pub drift: Vec<LocalMaterializationDrift>,
}

impl LocalRepositoryStatus {
    fn from_api(value: ApiLocalRepositoryStatus) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            is_clean: value.is_clean(),
            drift: value
                .drift()
                .iter()
                .map(LocalMaterializationDrift::from_api)
                .collect(),
        }
    }
}

/// Local plan request over one retained compilation.
#[napi(object, object_to_js = false, js_name = "LocalOneShotPlanRequest")]
pub struct LocalOneShotPlanRequest {
    /// Logical repository selected for planning.
    pub repository_id: String,
    /// Apply, explicit-adoption, or removal law.
    pub kind: OneShotPlanKind,
}

/// Flat transition counts derived from one complete local plan.
#[napi(object, object_from_js = false, js_name = "LocalOneShotPlanCounts")]
pub struct LocalOneShotPlanCounts {
    /// Number of observed repository targets.
    pub baseline: u32,
    /// Number of repository-surface operations.
    pub operations: u32,
    /// Total repository-surface and machine-local operations.
    pub total_operations: u32,
    /// Number of create transitions.
    pub create: u32,
    /// Number of explicit adoption transitions.
    pub adopt: u32,
    /// Number of replacement transitions.
    pub update: u32,
    /// Number of unchanged surfaces.
    pub unchanged: u32,
    /// Number of deletion transitions.
    pub delete: u32,
    /// Number of managed-block splice transitions.
    pub splice: u32,
    /// Number of deliberately deferred effects.
    pub deferred_effects: u32,
    /// Number of bound machine-local effects.
    pub machine_local_effects: u32,
}

impl From<ApiLocalOneShotPlanCounts> for LocalOneShotPlanCounts {
    fn from(value: ApiLocalOneShotPlanCounts) -> Self {
        Self {
            baseline: value.baseline(),
            operations: value.operations(),
            total_operations: value.total_operations(),
            create: value.create(),
            adopt: value.adopt(),
            update: value.update(),
            unchanged: value.unchanged(),
            delete: value.delete(),
            splice: value.splice(),
            deferred_effects: value.deferred_effects(),
            machine_local_effects: value.machine_local_effects(),
        }
    }
}

/// Complete deterministic local plan with privately retained host context.
#[napi(js_name = "LocalOneShotPlan")]
pub struct LocalOneShotPlan {
    inner: ApiLocalOneShotPlan,
}

#[napi]
impl LocalOneShotPlan {
    /// Requested plan operation.
    #[napi(getter)]
    pub fn kind(&self) -> OneShotPlanKind {
        self.inner.document().kind().into()
    }

    /// Selected logical repository.
    #[napi(getter)]
    pub fn repository_id(&self) -> String {
        self.inner.document().repository_id().to_owned()
    }

    /// Copy the canonical complete materialization-plan bytes.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.document().canonical_bytes().to_vec().into()
    }

    /// Return counts derived from the same API-owned plan.
    #[napi]
    pub fn counts(&self) -> LocalOneShotPlanCounts {
        self.inner.counts().into()
    }
}

/// Binding-safe facts for one atomically published saved plan.
#[napi(
    object,
    object_from_js = false,
    js_name = "LocalOneShotPlanPublication"
)]
pub struct LocalOneShotPlanPublication {
    /// Logical repository named by the published plan.
    pub repository_id: String,
    /// Confined filename under the selected output directory.
    pub relative_target: String,
    /// Ephemeral display spelling of the published target.
    pub display_target: String,
}

/// Local directories and plan class admitted for one materialization run.
#[napi(
    object,
    object_to_js = false,
    js_name = "LocalMaterializationAdmissionRequest"
)]
pub struct LocalMaterializationAdmissionRequest {
    /// Apply, explicit-adoption, or removal law.
    pub kind: OneShotPlanKind,
    /// Absolute directory containing the exact saved plans.
    pub saved_plan_directory: String,
    /// Absolute directory selected for immutable evidence.
    pub evidence_directory: String,
}

/// Path-free identity of one repository plan admitted before effects.
#[napi(
    object,
    object_from_js = false,
    js_name = "LocalMaterializationTargetAdmission"
)]
pub struct LocalMaterializationTargetAdmission {
    /// Logical repository whose plan was admitted.
    pub repository_id: String,
    /// Digest of the exact saved plan.
    pub plan_digest: String,
}

impl LocalMaterializationTargetAdmission {
    fn from_api(value: &ApiLocalMaterializationTargetAdmission) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            plan_digest: value.plan_digest().to_owned(),
        }
    }
}

/// Opaque admitted local run awaiting settlement or non-effectful drop.
#[napi(js_name = "LocalMaterializationAdmittedRun")]
pub struct LocalMaterializationAdmittedRun {
    inner: ApiLocalMaterializationAdmittedRun,
}

#[napi]
impl LocalMaterializationAdmittedRun {
    /// Requested apply, explicit-adoption, or removal journey.
    #[napi(getter)]
    pub fn kind(&self) -> OneShotPlanKind {
        self.inner.kind().into()
    }

    /// Ordered repository-plan identities admitted before effects.
    #[napi]
    pub fn targets(&self) -> Vec<LocalMaterializationTargetAdmission> {
        self.inner
            .targets()
            .iter()
            .map(LocalMaterializationTargetAdmission::from_api)
            .collect()
    }
}

/// Aggregate result of one application-admitted settlement.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationRunOutcome {
    /// Every selected repository settled with an immutable receipt.
    Succeeded,
    /// A successful prefix settled before a later no-effect failure.
    Partial,
    /// The first repository retained no materialization effect.
    FailedNoEffect,
}

impl From<ApiLocalMaterializationRunOutcome> for LocalMaterializationRunOutcome {
    fn from(value: ApiLocalMaterializationRunOutcome) -> Self {
        match value {
            ApiLocalMaterializationRunOutcome::Succeeded => Self::Succeeded,
            ApiLocalMaterializationRunOutcome::Partial => Self::Partial,
            ApiLocalMaterializationRunOutcome::FailedNoEffect => Self::FailedNoEffect,
        }
    }
}

/// Outcome of one repository settlement in project order.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationTargetOutcome {
    /// The immutable receipt preceded repository commit.
    Succeeded,
    /// The repository retained no materialization effect.
    FailedNoEffect,
}

impl From<ApiLocalMaterializationTargetOutcome> for LocalMaterializationTargetOutcome {
    fn from(value: ApiLocalMaterializationTargetOutcome) -> Self {
        match value {
            ApiLocalMaterializationTargetOutcome::Succeeded => Self::Succeeded,
            ApiLocalMaterializationTargetOutcome::FailedNoEffect => Self::FailedNoEffect,
        }
    }
}

/// Presentation-neutral facts for one attempted repository settlement.
#[napi(
    object,
    object_from_js = false,
    js_name = "LocalMaterializationTargetResult"
)]
pub struct LocalMaterializationTargetResult {
    /// Logical repository attempted by this row.
    pub repository_id: String,
    /// Proven per-repository outcome.
    pub outcome: LocalMaterializationTargetOutcome,
    /// Number of repository surfaces changed on success.
    pub changed_surface_count: u32,
    /// Number of effects kept outside the transaction.
    pub deferred_effect_count: u32,
    /// Whether the managed Git-exclude block changed.
    pub local_git_exclude_changed: bool,
    /// Canonical diagnostic bytes for a no-effect failure, when present.
    pub failure_diagnostic: Option<Buffer>,
}

impl LocalMaterializationTargetResult {
    fn from_api(value: &ApiLocalMaterializationTargetResult) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            outcome: value.outcome().into(),
            changed_surface_count: value.changed_surface_count(),
            deferred_effect_count: value.deferred_effect_count(),
            local_git_exclude_changed: value.local_git_exclude_changed(),
            failure_diagnostic: value
                .failure()
                .map(|diagnostic| diagnostic.canonical_bytes().to_vec().into()),
        }
    }
}

/// Binding-safe result of one settled repository-scoped run.
#[napi(
    object,
    object_from_js = false,
    js_name = "LocalMaterializationRunResult"
)]
pub struct LocalMaterializationRunResult {
    /// Requested plan operation.
    pub kind: OneShotPlanKind,
    /// Aggregate repository-scoped outcome.
    pub outcome: LocalMaterializationRunOutcome,
    /// Number of repository results included.
    pub target_count: u32,
    /// Number of repositories fully settled.
    pub succeeded_target_count: u32,
    /// Ordered result rows for attempted repositories.
    pub targets: Vec<LocalMaterializationTargetResult>,
    /// Canonical application-admitted `run-result.v1` bytes.
    pub canonical_bytes: Buffer,
}

impl LocalMaterializationRunResult {
    fn from_api(value: ApiLocalMaterializationRunResult) -> Self {
        Self {
            kind: value.kind().into(),
            outcome: value.outcome().into(),
            target_count: value.target_count(),
            succeeded_target_count: value.succeeded_target_count(),
            targets: value
                .targets()
                .iter()
                .map(LocalMaterializationTargetResult::from_api)
                .collect(),
            canonical_bytes: value.canonical_bytes().to_vec().into(),
        }
    }
}

/// Observation of the immutable receipt slot after an unknown outcome.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationReceiptObservation {
    /// Receipt bytes were not constructed before failure.
    NotConstructed,
    /// Exact admitted receipt bytes and mode are visible.
    Exact,
    /// No receipt is visible at the admitted identity.
    Absent,
    /// Different bytes or mode occupy the admitted identity.
    Different,
}

impl From<ApiLocalMaterializationReceiptObservation> for LocalMaterializationReceiptObservation {
    fn from(value: ApiLocalMaterializationReceiptObservation) -> Self {
        match value {
            ApiLocalMaterializationReceiptObservation::NotConstructed => Self::NotConstructed,
            ApiLocalMaterializationReceiptObservation::Exact => Self::Exact,
            ApiLocalMaterializationReceiptObservation::Absent => Self::Absent,
            ApiLocalMaterializationReceiptObservation::Different => Self::Different,
        }
    }
}

/// Trusted ownership-record relation to an admitted transition.
#[napi]
#[derive(Clone, Copy)]
pub enum LocalMaterializationOwnershipObservation {
    /// No trusted ownership record is present.
    Unowned,
    /// The trusted record matches the admitted pre-state.
    MatchesPrior,
    /// The trusted record matches the admitted post-state.
    MatchesPost,
    /// A trusted record matches neither admitted endpoint.
    Other,
}

impl From<ApiLocalMaterializationOwnershipObservation>
    for LocalMaterializationOwnershipObservation
{
    fn from(value: ApiLocalMaterializationOwnershipObservation) -> Self {
        match value {
            ApiLocalMaterializationOwnershipObservation::Unowned => Self::Unowned,
            ApiLocalMaterializationOwnershipObservation::MatchesPrior => Self::MatchesPrior,
            ApiLocalMaterializationOwnershipObservation::MatchesPost => Self::MatchesPost,
            ApiLocalMaterializationOwnershipObservation::Other => Self::Other,
        }
    }
}

/// Bounded read-back after settlement reported outcome-unknown.
#[napi(
    object,
    object_from_js = false,
    js_name = "LocalMaterializationReconciliation"
)]
pub struct LocalMaterializationReconciliation {
    /// Repository whose unknown settlement is inspected.
    pub repository_id: String,
    /// Exact admitted plan identity.
    pub plan_digest: String,
    /// Current immutable receipt-slot observation.
    pub receipt: LocalMaterializationReceiptObservation,
    /// Current trusted ownership-record relation.
    pub ownership: LocalMaterializationOwnershipObservation,
}

impl From<ApiLocalMaterializationReconciliation> for LocalMaterializationReconciliation {
    fn from(value: ApiLocalMaterializationReconciliation) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            plan_digest: value.plan_digest().to_owned(),
            receipt: value.receipt().into(),
            ownership: value.ownership().into(),
        }
    }
}

/// Durable-publication implementation compiled for this native target.
#[napi]
#[derive(Clone, Copy)]
pub enum DurablePublicationMechanism {
    /// Apple `F_FULLFSYNC`, then parent-directory `fsync`.
    AppleFullFsync,
    /// POSIX file and parent-directory `fsync`.
    UnixFsync,
    /// Rust `sync_all` for the file and parent directory.
    StandardSyncAll,
}

impl From<ApiDurablePublicationMechanism> for DurablePublicationMechanism {
    fn from(value: ApiDurablePublicationMechanism) -> Self {
        match value {
            ApiDurablePublicationMechanism::AppleFullFsync => Self::AppleFullFsync,
            ApiDurablePublicationMechanism::UnixFsync => Self::UnixFsync,
            ApiDurablePublicationMechanism::StandardSyncAll => Self::StandardSyncAll,
        }
    }
}

/// Compiled SDK and interface claims, not resident state or process health.
#[napi(object, object_from_js = false, js_name = "CompiledInterfaceIdentity")]
pub struct CompiledInterfaceIdentity {
    /// Canonical JCS executable-identity record.
    pub canonical_bytes: Buffer,
    /// SDK package version compiled into this addon.
    pub version: String,
    /// Interface protocol compiled into this addon.
    pub protocol: String,
    /// Operating-system identity compiled for this process.
    pub os: String,
    /// Architecture identity compiled for this process.
    pub arch: String,
    /// Durable-publication implementation compiled for this process.
    pub durable_publication: DurablePublicationMechanism,
}

impl From<ApiCompiledInterfaceIdentity> for CompiledInterfaceIdentity {
    fn from(value: ApiCompiledInterfaceIdentity) -> Self {
        Self {
            canonical_bytes: value.canonical_bytes().to_vec().into(),
            version: value.version().to_owned(),
            protocol: value.protocol().to_owned(),
            os: value.os().to_owned(),
            arch: value.arch().to_owned(),
            durable_publication: value.durable_publication().into(),
        }
    }
}

#[napi]
impl Pale {
    /// Resolve, stably capture, and compile one local workspace selection.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the host
    /// locator, source capture, selectors, or canonical compiler rejects input.
    #[napi]
    pub fn compile_local_one_shot(
        &self,
        env: Env,
        request: LocalOneShotCompilationRequest,
    ) -> Result<LocalOneShotCompilation> {
        self.api
            .compile_local_one_shot(ApiLocalOneShotCompilationRequest::new(
                LocalWorkspaceLocator::new(request.workspace_manifest),
                request.project_id,
                request.execution_workspace_id,
                request.machine_id,
            ))
            .map(|inner| LocalOneShotCompilation { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Inspect Package releases selected by one local workspace entry.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when local
    /// selection, bounded Package capture, or Package admission fails.
    #[napi]
    pub fn inspect_local_package_distribution(
        &self,
        env: Env,
        request: LocalPackageDistributionRequest,
    ) -> Result<PackageDistributionInspection> {
        self.api
            .inspect_local_package_distribution(ApiLocalPackageDistributionRequest::new(
                LocalWorkspaceLocator::new(request.workspace_manifest),
                request.project_id,
                request.execution_workspace_id,
                request.machine_id,
            ))
            .map(PackageDistributionInspection::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Read one confined local file into a path-free action request.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the root,
    /// confined relative path, file type, or byte ceiling is rejected.
    #[napi]
    pub fn capture_local_native_action_request(
        &self,
        env: Env,
        request: LocalNativeActionInputRequest,
    ) -> Result<CapturedNativeActionRequest> {
        self.api
            .capture_local_native_action_request(ApiLocalNativeActionInputRequest::new(
                request.action_id,
                request.root,
                request.relative_path,
            ))
            .map(|inner| CapturedNativeActionRequest { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Invoke one captured request through the same pure native-action owner.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the action,
    /// tagged input, or pure execution fails API-owned admission.
    #[napi]
    pub fn invoke_captured_native_action(
        &self,
        env: Env,
        request: &CapturedNativeActionRequest,
    ) -> Result<Buffer> {
        self.api
            .invoke_native_action(request.inner.clone())
            .map(|execution| execution.canonical_bytes().to_vec().into())
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Compare one local repository with its admitted desired projection.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when selection,
    /// lineage, baseline, Git authority, or comparison admission fails.
    #[napi]
    pub fn inspect_local_repository_status(
        &self,
        env: Env,
        compilation: &LocalOneShotCompilation,
        repository_id: String,
    ) -> Result<LocalRepositoryStatus> {
        self.api
            .inspect_local_repository_status(&compilation.inner, repository_id)
            .map(LocalRepositoryStatus::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Reconstruct one complete local plan from trusted observations.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when repository,
    /// lineage, baseline, Git authority, or plan admission fails.
    #[napi]
    pub fn plan_local_one_shot(
        &self,
        env: Env,
        compilation: &LocalOneShotCompilation,
        request: LocalOneShotPlanRequest,
    ) -> Result<LocalOneShotPlan> {
        self.api
            .plan_local_one_shot(
                &compilation.inner,
                ApiLocalOneShotPlanRequest::new(request.repository_id, request.kind.into()),
            )
            .map(|inner| LocalOneShotPlan { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Publish one complete plan into a confined local directory.
    ///
    /// This method is the explicit native-effect boundary for saved-plan
    /// publication. Constructing the request or plan does not invoke it.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when directory
    /// admission, retained identity, or atomic publication fails.
    #[napi]
    pub fn publish_local_one_shot_plan(
        &self,
        env: Env,
        plan: &LocalOneShotPlan,
        output_directory: String,
    ) -> Result<LocalOneShotPlanPublication> {
        self.api
            .publish_local_one_shot_plan(&plan.inner, output_directory)
            .map(|value| LocalOneShotPlanPublication {
                repository_id: value.repository_id().to_owned(),
                relative_target: value.relative_target().to_owned(),
                display_target: value.display_target().to_owned(),
            })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Admit saved plans and target effects without writing.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when directory
    /// spelling, saved-plan equality, observations, or Git-local preparation fails.
    #[napi]
    pub fn admit_local_materialization_run(
        &self,
        env: Env,
        compilation: &LocalOneShotCompilation,
        request: LocalMaterializationAdmissionRequest,
    ) -> Result<LocalMaterializationAdmittedRun> {
        self.api
            .admit_local_materialization_run(
                &compilation.inner,
                ApiLocalMaterializationAdmissionRequest::new(
                    request.kind.into(),
                    request.saved_plan_directory,
                    request.evidence_directory,
                ),
            )
            .map(|inner| LocalMaterializationAdmittedRun { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Settle one admitted run in repository order.
    ///
    /// This is an explicit native-effect boundary. Executable identity and every
    /// receipt slot are admitted before the first repository mutation.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes for pre-effect
    /// rejection or outcome-unknown settlement requiring read-back.
    #[napi]
    pub fn settle_local_materialization_run(
        &self,
        env: Env,
        run: &mut LocalMaterializationAdmittedRun,
    ) -> Result<LocalMaterializationRunResult> {
        self.api
            .settle_local_materialization_run(&mut run.inner)
            .map(LocalMaterializationRunResult::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Inspect bounded read-back after an outcome-unknown settlement.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the run is
    /// not unknown or retained receipt/ownership evidence cannot be observed.
    #[napi]
    pub fn inspect_local_materialization_reconciliation(
        &self,
        env: Env,
        run: &LocalMaterializationAdmittedRun,
    ) -> Result<LocalMaterializationReconciliation> {
        self.api
            .inspect_local_materialization_reconciliation(&run.inner)
            .map(Into::into)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Report compiled SDK and interface claims without reading an executable.
    ///
    /// The record establishes neither executable-byte identity nor resident state.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes if the embedded
    /// runtime schema or canonical record is incoherent.
    #[napi]
    pub fn compiled_interface_identity(&self, env: Env) -> Result<CompiledInterfaceIdentity> {
        self.api
            .compiled_interface_identity()
            .map(Into::into)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Verify the current host executable against one captured workspace binding.
    ///
    /// This checks the Node executable, not the loaded addon's bytes. Success
    /// returns the compiled interface claims; it establishes no resident state
    /// or process health and starts no service.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when no runtime
    /// is bound or executable bytes, version, protocol, or identity disagree.
    #[napi]
    pub fn verify_current_executable_binding(
        &self,
        env: Env,
        compilation: &LocalOneShotCompilation,
    ) -> Result<CompiledInterfaceIdentity> {
        self.api
            .verify_current_executable_binding(&compilation.inner)
            .map(Into::into)
            .map_err(|error| diagnostic_error(&env, error))
    }
}
