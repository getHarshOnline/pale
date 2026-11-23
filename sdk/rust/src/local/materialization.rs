//! Local status, deterministic planning, and repository-scoped settlement.
//!
//! Host observation is composed here, while transition, ownership, adoption,
//! removal, and saved-plan laws remain application-owned. Public values retain
//! logical identities and canonical plan bytes only; machine paths and adapter
//! capabilities remain private.

mod effect;
#[path = "git_ignore.rs"]
mod git_ignore;

pub use effect::{
    LocalMaterializationAdmissionRequest, LocalMaterializationAdmittedRun,
    LocalMaterializationOwnershipObservation, LocalMaterializationReceiptObservation,
    LocalMaterializationReconciliation, LocalMaterializationRunOutcome,
    LocalMaterializationRunResult, LocalMaterializationTargetAdmission,
    LocalMaterializationTargetOutcome, LocalMaterializationTargetResult,
    LocalOneShotPlanPublication,
};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pale_application::ApplicationFailureDisposition;
use pale_core::records::{
    BaselineEntry, BaselineStateV1, DiagnosticStageV1, LogicalRepositoryIdV1,
    MaterializationStateV1, MATERIALIZATION_STATE_PATH,
};

use super::{local_host_failure, LocalOneShotCompilation};
use crate::error::ApiError;
use crate::{
    MachineLocalEffectCandidate, OneShotBaselineEntry, OneShotBaselineObservation,
    OneShotBoundPlan, OneShotPlanKind, OneShotPlanningRequest, OneShotPriorMaterialization,
    PaleApi,
};

/// Closed repository or machine-local transition shown by local status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

/// Ownership-record transition required by local status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMaterializationOwnershipChange {
    /// Establish the first trusted ownership state.
    Initialize,
    /// Advance an existing trusted ownership state.
    Advance,
}

/// Closed machine-local policy effect visible through local status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMaterializationMachineEffectKind {
    /// Managed protection in the checkout's verified common Git exclude file.
    GitExcludeProtection,
}

impl LocalMaterializationMachineEffectKind {
    const fn status_identifier(self) -> &'static str {
        match self {
            Self::GitExcludeProtection => "git_exclude_protection",
        }
    }

    const fn result_identifier(self) -> &'static str {
        match self {
            Self::GitExcludeProtection => "git-exclude-protection",
        }
    }
}

/// One exact reason an observed local repository differs from its projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalMaterializationDrift {
    /// One repository-relative generated surface differs.
    Surface {
        /// Confined repository-relative target.
        relative_path: String,
        /// Transition required to reach desired state.
        action: LocalMaterializationAction,
    },
    /// The trusted ownership record must be initialized or advanced.
    Ownership {
        /// Required ownership-state transition.
        change: LocalMaterializationOwnershipChange,
    },
    /// One machine-local policy file differs.
    MachineLocal {
        /// Closed machine-local effect kind.
        kind: LocalMaterializationMachineEffectKind,
        /// Path relative to its verified machine-local root.
        relative_path: String,
        /// Transition required to reach desired state.
        action: LocalMaterializationAction,
    },
}

impl LocalMaterializationDrift {
    /// Return the complete stable label used by local status presentation.
    ///
    /// Surface drift retains its observed relative target, ownership drift names
    /// the canonical materialization-state record, and machine-local drift is
    /// qualified by its closed effect kind. Leaves render this label but do not
    /// reconstruct its semantic coordinate.
    #[must_use]
    pub fn status_label(&self) -> String {
        match self {
            Self::Surface { relative_path, .. } => relative_path.clone(),
            Self::Ownership { .. } => MATERIALIZATION_STATE_PATH.to_owned(),
            Self::MachineLocal {
                kind,
                relative_path,
                ..
            } => format!("{}:{relative_path}", kind.status_identifier()),
        }
    }
}

/// Complete read-only comparison for one local repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalRepositoryStatus {
    repository_id: String,
    drift: Vec<LocalMaterializationDrift>,
}

/// Read-only comparison of every selected repository with its generated state.
///
/// Repositories are ordered by ID. The
/// canonical bytes contain no host paths and grant no planning or mutation
/// authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalWorkspaceStatus {
    clean: bool,
    repositories: Vec<LocalRepositoryStatus>,
    canonical_bytes: Vec<u8>,
}

impl LocalWorkspaceStatus {
    /// Whether every selected repository matches its generated state.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.clean
    }

    /// Repository results in stable ID order.
    #[must_use]
    pub fn repositories(&self) -> &[LocalRepositoryStatus] {
        &self.repositories
    }

    /// Borrow the canonical structured result.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalWorkspaceStatusProjection<'a> {
    clean: bool,
    repositories: Vec<LocalRepositoryStatusProjection<'a>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRepositoryStatusProjection<'a> {
    repository_id: &'a str,
    drift: Vec<LocalMaterializationDriftProjection<'a>>,
}

#[derive(serde::Serialize)]
struct LocalMaterializationDriftProjection<'a> {
    kind: &'static str,
    path: &'a str,
    action: &'static str,
}

impl LocalRepositoryStatus {
    /// Logical repository selected from the admitted compilation.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Whether every observed surface, ownership record, and local policy file matches.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.drift.is_empty()
    }

    /// Deterministically ordered drift partition.
    #[must_use]
    pub fn drift(&self) -> &[LocalMaterializationDrift] {
        &self.drift
    }
}

/// Local plan request over one retained one-shot compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalOneShotPlanRequest {
    repository_id: String,
    kind: OneShotPlanKind,
}

impl LocalOneShotPlanRequest {
    /// Select one repository and apply, explicit-adoption, or removal law.
    #[must_use]
    pub fn new(repository_id: String, kind: OneShotPlanKind) -> Self {
        Self {
            repository_id,
            kind,
        }
    }
}

/// Flat transition counts for one complete local plan document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalOneShotPlanCounts {
    baseline: u32,
    operations: u32,
    total_operations: u32,
    create: u32,
    adopt: u32,
    update: u32,
    unchanged: u32,
    delete: u32,
    splice: u32,
    deferred_effects: u32,
    machine_local_effects: u32,
}

macro_rules! count_accessor {
    ($name:ident, $field:ident, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub const fn $name(self) -> u32 {
            self.$field
        }
    };
}

impl LocalOneShotPlanCounts {
    count_accessor!(baseline, baseline, "Number of observed repository targets.");
    count_accessor!(
        operations,
        operations,
        "Number of repository-surface operations."
    );
    count_accessor!(
        total_operations,
        total_operations,
        "Total repository-surface and machine-local operations."
    );
    count_accessor!(create, create, "Number of create transitions.");
    count_accessor!(adopt, adopt, "Number of explicit adoption transitions.");
    count_accessor!(update, update, "Number of replacement transitions.");
    count_accessor!(unchanged, unchanged, "Number of unchanged surfaces.");
    count_accessor!(delete, delete, "Number of deletion transitions.");
    count_accessor!(
        splice,
        splice,
        "Number of managed-block splice transitions."
    );
    count_accessor!(
        deferred_effects,
        deferred_effects,
        "Number of declared effects intentionally deferred from this transaction."
    );
    count_accessor!(
        machine_local_effects,
        machine_local_effects,
        "Number of bound machine-local effects included in the complete plan."
    );
}

/// Complete deterministic local plan and its privately retained host context.
///
/// [`Self::document`] contains the canonical Pale plan after machine-local
/// observations are bound. The retained roots are inaccessible to callers and
/// grant no saved-plan admission or mutation authority.
pub struct LocalOneShotPlan {
    document: OneShotBoundPlan,
    counts: LocalOneShotPlanCounts,
    repository_root: PathBuf,
    lineage_root: PathBuf,
    git_metadata_root: Option<PathBuf>,
}

impl std::fmt::Debug for LocalOneShotPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalOneShotPlan")
            .field("document", &self.document)
            .field("counts", &self.counts)
            .finish_non_exhaustive()
    }
}

impl LocalOneShotPlan {
    /// Borrow the canonical complete plan document.
    #[must_use]
    pub const fn document(&self) -> &OneShotBoundPlan {
        &self.document
    }

    /// Return transition counts derived from the same application-owned plan.
    #[must_use]
    pub const fn counts(&self) -> LocalOneShotPlanCounts {
        self.counts
    }
}

impl PaleApi {
    /// Compare every selected local repository with its generated state.
    ///
    /// Each repository uses [`Self::inspect_local_repository_status`]. Results
    /// are sorted by logical repository identifier and encoded once as a
    /// canonical structured value. This operation reads local state but writes
    /// no file and creates no plan or mutation authority.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when any repository comparison or canonical result
    /// encoding fails.
    pub fn inspect_local_workspace_status(
        &self,
        compilation: &LocalOneShotCompilation,
    ) -> Result<LocalWorkspaceStatus, ApiError> {
        let mut repository_ids = compilation
            .summary()
            .repositories()
            .iter()
            .map(|repository| repository.repository_id().to_owned())
            .collect::<Vec<_>>();
        repository_ids.sort();
        let repositories = repository_ids
            .into_iter()
            .map(|repository_id| self.inspect_local_repository_status(compilation, repository_id))
            .collect::<Result<Vec<_>, _>>()?;
        let clean = repositories.iter().all(LocalRepositoryStatus::is_clean);
        let projection = project_workspace_status(clean, &repositories);
        let canonical_bytes = pale_core::canon::to_canonical_string(&projection)
            .map(String::into_bytes)
            .map_err(|error| local_canon_failure(self, error))?;
        Ok(LocalWorkspaceStatus {
            clean,
            repositories,
            canonical_bytes,
        })
    }

    /// Compare one local repository with the admitted desired projection.
    ///
    /// Trusted ownership is accepted only when the repository-local state and
    /// independently rooted lineage agree. Repository and Git-local bytes are
    /// observed through confined adapters; the application status algebra owns
    /// the drift partition. No plan or mutation authority is created.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when repository selection, lineage, baseline, Git
    /// common-directory authority, or application comparison is rejected.
    pub fn inspect_local_repository_status(
        &self,
        compilation: &LocalOneShotCompilation,
        repository_id: String,
    ) -> Result<LocalRepositoryStatus, ApiError> {
        let host = LocalMaterializationHost::resolve(self, compilation, &repository_id)?;
        let prior = host.read_prior(self, compilation)?;
        let input = pale_application::plan::RepositoryMaterializationInput::for_repository(
            compilation.compilation.application(),
            &host.repository_id,
        )
        .map_err(|error| local_plan_failure(self, error))?;
        let targets = input
            .required_observation_targets(pale_core::records::PlanKind::Apply, prior.as_ref())
            .map_err(|error| local_plan_failure(self, error))?;
        let baseline = pale_adapter_fs::read_baseline(
            &host.repository_root,
            &targets,
            pale_adapter_fs::MATERIALIZED_SURFACE_READ_LIMIT,
        )
        .map_err(|error| local_fs_failure(self, error))?;
        let comparison = pale_application::plan::compare_reconciled_materialization(
            input,
            prior.as_ref(),
            baseline,
        )
        .map_err(|error| local_plan_failure(self, error))?;
        let effects = git_ignore::plan_protection(
            &host.repository_root,
            host.git_metadata_root.as_deref(),
            comparison.local_only_targets(),
        )
        .map_err(|error| local_ignore_failure(self, error))?;
        let root = pale_adapter_fs::ConfinedRoot::open(&host.repository_root)
            .map_err(|error| local_fs_failure(self, error))?;
        let mut changed_owned_paths = BTreeSet::new();
        for operation in comparison.unchanged_operations() {
            let matches = pale_adapter_fs::owned_surface_matches(
                &root,
                &operation.rel_path,
                operation.transition.ownership(),
                operation.transition.content_sha256(),
                operation.mode,
            )
            .map_err(|error| local_fs_failure(self, error))?;
            if !matches {
                changed_owned_paths.insert(operation.rel_path.clone());
            }
        }
        root.verify_path_identity()
            .map_err(|error| local_fs_failure(self, error))?;
        let status = comparison
            .complete(effects, &changed_owned_paths)
            .map_err(|error| local_plan_failure(self, error))?;
        Ok(project_status(status))
    }

    /// Reconstruct one complete local plan from trusted current observations.
    ///
    /// Apply, explicit adoption, and removal reuse the path-free planning API
    /// after this method captures trusted lineage and a confined baseline. Git
    /// protection is derived from the verified common directory and bound as a
    /// machine-local effect. The returned bytes are deterministic but remain a
    /// candidate until saved-plan admission succeeds separately.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for any rejected repository, lineage, baseline,
    /// application plan, Git authority, or canonical machine-effect projection.
    pub fn plan_local_one_shot(
        &self,
        compilation: &LocalOneShotCompilation,
        request: LocalOneShotPlanRequest,
    ) -> Result<LocalOneShotPlan, ApiError> {
        let LocalOneShotPlanRequest {
            repository_id,
            kind,
        } = request;
        let host = LocalMaterializationHost::resolve(self, compilation, &repository_id)?;
        let prior = host.read_prior(self, compilation)?;
        let prior = project_prior(prior).map_err(|error| local_canon_failure(self, error))?;
        let targets = self.one_shot_observation_targets(
            compilation.compilation(),
            repository_id.clone(),
            kind,
            &prior,
        )?;
        let baseline = pale_adapter_fs::read_baseline(
            &host.repository_root,
            &targets
                .iter()
                .map(|target| target.relative_path().to_owned())
                .collect::<Vec<_>>(),
            pale_adapter_fs::MATERIALIZED_SURFACE_READ_LIMIT,
        )
        .map_err(|error| local_fs_failure(self, error))?;
        let candidate = self.plan_one_shot(
            compilation.compilation(),
            OneShotPlanningRequest::new(
                repository_id,
                kind,
                prior,
                baseline.into_iter().map(project_baseline).collect(),
            ),
        )?;
        let effects = match kind {
            OneShotPlanKind::Apply | OneShotPlanKind::Adopt => git_ignore::plan_protection(
                &host.repository_root,
                host.git_metadata_root.as_deref(),
                candidate.local_only_targets(),
            ),
            OneShotPlanKind::Remove => git_ignore::plan_unprotection(
                &host.repository_root,
                host.git_metadata_root.as_deref(),
            ),
        }
        .map_err(|error| local_ignore_failure(self, error))?
        .into_iter()
        .map(|effect| {
            pale_core::canon::to_canonical_string(&effect)
                .map(|bytes| MachineLocalEffectCandidate::new(bytes.into_bytes()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| local_canon_failure(self, error))?;
        let machine_local_effects = u32::try_from(effects.len()).map_err(|_| {
            local_plan_failure(
                self,
                pale_application::plan::PlanError::PlanInvariant {
                    coordinate: "machine-local-effect-count",
                    detail: "effect count exceeds the local plan projection width".to_owned(),
                },
            )
        })?;
        let operations = candidate.operation_count();
        let total_operations = operations
            .checked_add(machine_local_effects)
            .ok_or_else(|| {
                local_plan_failure(
                    self,
                    pale_application::plan::PlanError::PlanInvariant {
                        coordinate: "local-operation-count",
                        detail: "complete operation count exceeds the local plan projection width"
                            .to_owned(),
                    },
                )
            })?;
        let counts = LocalOneShotPlanCounts {
            baseline: candidate.baseline_count(),
            operations,
            total_operations,
            create: candidate.create_count(),
            adopt: candidate.adopt_count(),
            update: candidate.update_count(),
            unchanged: candidate.unchanged_count(),
            delete: candidate.delete_count(),
            splice: candidate.splice_count(),
            deferred_effects: candidate.deferred_effect_count(),
            machine_local_effects,
        };
        let document = self.bind_one_shot_machine_effects(candidate, effects)?;
        Ok(LocalOneShotPlan {
            document,
            counts,
            repository_root: host.repository_root,
            lineage_root: host.lineage_root,
            git_metadata_root: host.git_metadata_root,
        })
    }
}

struct LocalMaterializationHost {
    repository_id: LogicalRepositoryIdV1,
    repository_root: PathBuf,
    lineage_root: PathBuf,
    git_metadata_root: Option<PathBuf>,
}

impl LocalMaterializationHost {
    fn resolve(
        api: &PaleApi,
        compilation: &LocalOneShotCompilation,
        repository_id: &str,
    ) -> Result<Self, ApiError> {
        let repository_id = LogicalRepositoryIdV1::parse(repository_id.to_owned())
            .map_err(|error| local_identifier_failure(api, error))?;
        let repository_root = compilation
            .host
            .repository_root(repository_id.as_str())
            .ok_or_else(|| local_host_context_failure(api, "repository root is not selected"))?
            .to_path_buf();
        let binding = compilation.host.materialization_lineage().ok_or_else(|| {
            local_host_context_failure(api, "materialization lineage is not selected")
        })?;
        let lineage_root = pale_application::workspace::verify_materialization_lineage_binding(
            &pale_adapter_fs::LocalMachineFilesystem,
            binding,
        )
        .map_err(|error| local_workspace_failure(api, error))?;
        let git_metadata_root = compilation
            .host
            .git_metadata_root(repository_id.as_str())
            .map(Path::to_path_buf);
        Ok(Self {
            repository_id,
            repository_root,
            lineage_root,
            git_metadata_root,
        })
    }

    fn read_prior(
        &self,
        api: &PaleApi,
        compilation: &LocalOneShotCompilation,
    ) -> Result<Option<MaterializationStateV1>, ApiError> {
        let identity = pale_application::plan::materialization_contract_identity(
            compilation.compilation.application(),
        )
        .map_err(|error| local_contract_failure(api, error))?;
        pale_adapter_fs::read_materialization_state(
            &self.repository_root,
            &self.lineage_root,
            &self.repository_id,
            &identity,
        )
        .map_err(|error| local_fs_failure(api, error))
    }
}

fn project_prior(
    prior: Option<MaterializationStateV1>,
) -> Result<OneShotPriorMaterialization, pale_core::canon::CanonError> {
    prior.map_or(Ok(OneShotPriorMaterialization::Unowned), |state| {
        pale_core::canon::to_canonical_string(&state).map(|canonical_state| {
            OneShotPriorMaterialization::Owned {
                canonical_state: canonical_state.into_bytes(),
            }
        })
    })
}

fn project_baseline(entry: BaselineEntry) -> OneShotBaselineEntry {
    let observation = match entry.observation {
        BaselineStateV1::Absent {} => OneShotBaselineObservation::Absent,
        BaselineStateV1::Present { sha256, mode } => OneShotBaselineObservation::Present {
            sha256: sha256.to_string(),
            unix_mode: mode.get(),
        },
    };
    OneShotBaselineEntry::new(entry.rel_path, observation)
}

fn project_status(
    status: pale_application::plan::RepositoryMaterializationStatus,
) -> LocalRepositoryStatus {
    LocalRepositoryStatus {
        repository_id: status.repository_id().to_string(),
        drift: status
            .reasons()
            .iter()
            .map(|reason| match reason {
                pale_application::plan::MaterializationDriftReason::Surface {
                    relative_path,
                    action,
                } => LocalMaterializationDrift::Surface {
                    relative_path: relative_path.clone(),
                    action: project_action(*action),
                },
                pale_application::plan::MaterializationDriftReason::OwnershipState { change } => {
                    LocalMaterializationDrift::Ownership {
                        change: match change {
                            pale_application::plan::MaterializationOwnershipChange::Initialize => {
                                LocalMaterializationOwnershipChange::Initialize
                            }
                            pale_application::plan::MaterializationOwnershipChange::Advance => {
                                LocalMaterializationOwnershipChange::Advance
                            }
                        },
                    }
                }
                pale_application::plan::MaterializationDriftReason::MachineLocal {
                    kind,
                    relative_path,
                    action,
                } => LocalMaterializationDrift::MachineLocal {
                    kind: project_machine_effect_kind(*kind),
                    relative_path: relative_path.clone(),
                    action: project_action(*action),
                },
            })
            .collect(),
    }
}

fn project_workspace_status(
    clean: bool,
    repositories: &[LocalRepositoryStatus],
) -> LocalWorkspaceStatusProjection<'_> {
    LocalWorkspaceStatusProjection {
        clean,
        repositories: repositories
            .iter()
            .map(|status| LocalRepositoryStatusProjection {
                repository_id: status.repository_id(),
                drift: status.drift().iter().map(project_status_drift).collect(),
            })
            .collect(),
    }
}

fn project_status_drift(
    drift: &LocalMaterializationDrift,
) -> LocalMaterializationDriftProjection<'_> {
    match drift {
        LocalMaterializationDrift::Surface {
            relative_path,
            action,
        } => LocalMaterializationDriftProjection {
            kind: "surface",
            path: relative_path,
            action: project_status_action(*action),
        },
        LocalMaterializationDrift::Ownership { change } => LocalMaterializationDriftProjection {
            kind: "ownership",
            path: MATERIALIZATION_STATE_PATH,
            action: match change {
                LocalMaterializationOwnershipChange::Initialize => "initialize",
                LocalMaterializationOwnershipChange::Advance => "advance",
            },
        },
        LocalMaterializationDrift::MachineLocal {
            kind,
            relative_path,
            action,
        } => LocalMaterializationDriftProjection {
            kind: kind.result_identifier(),
            path: relative_path,
            action: project_status_action(*action),
        },
    }
}

const fn project_status_action(action: LocalMaterializationAction) -> &'static str {
    match action {
        LocalMaterializationAction::Create => "create",
        LocalMaterializationAction::Adopt => "adopt",
        LocalMaterializationAction::Update => "update",
        LocalMaterializationAction::Delete => "delete",
        LocalMaterializationAction::Splice => "splice",
    }
}

fn project_machine_effect_kind(
    kind: pale_core::records::MachineLocalEffectKind,
) -> LocalMaterializationMachineEffectKind {
    match kind {
        pale_core::records::MachineLocalEffectKind::GitExcludeProtection => {
            LocalMaterializationMachineEffectKind::GitExcludeProtection
        }
    }
}

fn project_action(action: pale_core::records::SurfaceAction) -> LocalMaterializationAction {
    match action {
        pale_core::records::SurfaceAction::Create => LocalMaterializationAction::Create,
        pale_core::records::SurfaceAction::Adopt => LocalMaterializationAction::Adopt,
        pale_core::records::SurfaceAction::Update => LocalMaterializationAction::Update,
        pale_core::records::SurfaceAction::Delete => LocalMaterializationAction::Delete,
        pale_core::records::SurfaceAction::Splice => LocalMaterializationAction::Splice,
        pale_core::records::SurfaceAction::Unchanged => {
            unreachable!("unchanged transitions are absent from drift rows")
        }
    }
}

fn local_plan_failure(api: &PaleApi, error: pale_application::plan::PlanError) -> ApiError {
    let disposition = error.failure_disposition();
    failure(
        api,
        error,
        disposition,
        "local_materialization_plan_rejected",
    )
}

fn local_workspace_failure(
    api: &PaleApi,
    error: pale_application::workspace::WorkspaceError,
) -> ApiError {
    let disposition = error.failure_disposition();
    failure(
        api,
        error,
        disposition,
        "local_materialization_host_rejected",
    )
}

fn local_fs_failure(api: &PaleApi, error: pale_adapter_fs::FsError) -> ApiError {
    failure(
        api,
        error,
        ApplicationFailureDisposition::Operational,
        "local_materialization_observation_failed",
    )
}

fn local_ignore_failure(api: &PaleApi, error: git_ignore::LocalIgnoreError) -> ApiError {
    failure(
        api,
        error,
        ApplicationFailureDisposition::Operational,
        "local_git_protection_observation_failed",
    )
}

fn local_identifier_failure(
    api: &PaleApi,
    error: pale_core::records::InvalidWorkspaceLanguageId,
) -> ApiError {
    failure(
        api,
        error,
        ApplicationFailureDisposition::InvalidInput,
        "local_repository_selector_rejected",
    )
}

fn local_contract_failure(api: &PaleApi, error: pale_contracts::SchemaError) -> ApiError {
    failure(
        api,
        error,
        ApplicationFailureDisposition::Internal,
        "local_materialization_contract_incoherent",
    )
}

fn local_canon_failure(api: &PaleApi, error: pale_core::canon::CanonError) -> ApiError {
    failure(
        api,
        error,
        ApplicationFailureDisposition::Internal,
        "local_materialization_projection_failed",
    )
}

#[derive(Debug, thiserror::Error)]
#[error("local materialization host context is incomplete: {0}")]
struct LocalHostContextError(&'static str);

fn local_host_context_failure(api: &PaleApi, detail: &'static str) -> ApiError {
    failure(
        api,
        LocalHostContextError(detail),
        ApplicationFailureDisposition::Blocked,
        "local_materialization_host_unbound",
    )
}

fn failure<E>(
    api: &PaleApi,
    error: E,
    disposition: ApplicationFailureDisposition,
    code: &'static str,
) -> ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    local_host_failure(
        &api.diagnostic_identity,
        error,
        disposition,
        DiagnosticStageV1::Planning,
        code,
        "local materialization could not be reconstructed from trusted observations",
    )
}
