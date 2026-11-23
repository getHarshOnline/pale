//! Run repository inspection, plan publication, and file changes through the SDK.
//!
//! Each successful pre-settlement transition returns the one owner required by
//! the next transition. The dispatcher keeps no handle table, clones no local
//! compilation, and performs no terminal presentation. A rejected settlement
//! returns both the exact API error and the mutated run so an explicitly
//! selected reconciliation read can still use that owner. Saved-plan
//! publication reports its completed prefix and the repository where the
//! ordered sequence stopped. It never treats uncertainty as authorization to
//! repeat publication.

use pale_sdk::{
    ApiError, LocalMaterializationAdmissionRequest, LocalMaterializationAdmittedRun,
    LocalMaterializationReconciliation, LocalMaterializationRunResult, LocalOneShotCompilation,
    LocalOneShotPlanCounts, LocalOneShotPlanPublication, LocalOneShotPlanRequest,
    LocalRepositoryStatus, LocalWorkspaceStatus, OneShotPlanKind, PaleApi,
};

/// Save one plan for every repository in the compilation's stable order.
///
/// Publication writes plan files. It does not apply those plans or change any
/// target repository.
pub struct LocalSavedPlanPublicationRequest {
    compilation: Box<LocalOneShotCompilation>,
    kind: OneShotPlanKind,
    output_directory: String,
}

impl LocalSavedPlanPublicationRequest {
    /// Store the retained compilation, plan kind, and requested output directory.
    ///
    /// Construction does not inspect the directory. Publication requires an
    /// absolute path without a parent-directory component, then opens or creates
    /// that directory through the filesystem adapter.
    #[must_use]
    pub fn new(
        compilation: Box<LocalOneShotCompilation>,
        kind: OneShotPlanKind,
        output_directory: String,
    ) -> Self {
        Self {
            compilation,
            kind,
            output_directory,
        }
    }
}

impl std::fmt::Debug for LocalSavedPlanPublicationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalSavedPlanPublicationRequest")
            .field("kind", &self.kind)
            .field("output_directory_byte_len", &self.output_directory.len())
            .finish_non_exhaustive()
    }
}

/// Counts and file facts returned after one saved plan was published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalSavedPlanPublication {
    counts: LocalOneShotPlanCounts,
    publication: LocalOneShotPlanPublication,
}

impl LocalSavedPlanPublication {
    /// Exact operation counts from the plan that was saved.
    #[must_use]
    pub const fn counts(&self) -> LocalOneShotPlanCounts {
        self.counts
    }

    /// Repository identity and paths returned by the SDK after publication.
    #[must_use]
    pub const fn publication(&self) -> &LocalOneShotPlanPublication {
        &self.publication
    }
}

/// Result of saving plans in stable repository order.
///
/// A stopped batch retains the exact SDK error. An outcome-unknown diagnostic
/// does not authorize repeating publication at the stop coordinate.
pub enum LocalSavedPlanPublicationDisposition {
    /// Every repository plan was saved.
    Complete {
        /// Retained compilation owner for a later local step.
        compilation: Box<LocalOneShotCompilation>,
        /// Every completed publication in stable repository order.
        publications: Vec<LocalSavedPlanPublication>,
    },
    /// Publication stopped while processing one repository in stable order.
    Stopped {
        /// Retained compilation owner for an explicitly selected later step.
        compilation: Box<LocalOneShotCompilation>,
        /// Completed publications before the stop, in stable order.
        publications: Vec<LocalSavedPlanPublication>,
        /// Repository at which the ordered batch stopped.
        ///
        /// This does not claim that publication is absent. The exact diagnostic
        /// establishes whether the target is absent, visible, or uncertain.
        stopped_repository_id: String,
        /// Exact SDK failure, including its effect certainty and retry posture.
        error: ApiError,
    },
}

impl std::fmt::Debug for LocalSavedPlanPublicationDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (state, publications) = match self {
            Self::Complete { publications, .. } => ("Complete", publications.len()),
            Self::Stopped { publications, .. } => ("Stopped", publications.len()),
        };
        formatter
            .debug_struct("LocalSavedPlanPublicationDisposition")
            .field("state", &state)
            .field("completed_publications", &publications)
            .finish_non_exhaustive()
    }
}

/// One local inspection, planning, or materialization transition.
///
/// Opaque owners move through this enum only in process. They are not
/// serializable continuation tokens and cannot be reconstructed from projected
/// summaries.
pub enum LocalMaterializationCommand {
    /// Compare every repository through the exact retained local compilation.
    InspectWorkspace {
        /// Local compilation consumed by this step and returned on success.
        compilation: Box<LocalOneShotCompilation>,
    },
    /// Compare one repository through the exact retained local compilation.
    InspectRepository {
        /// Local compilation consumed by this step and returned on success.
        compilation: Box<LocalOneShotCompilation>,
        /// Logical repository selected from the compilation.
        repository_id: String,
    },
    /// Save every repository plan through the same retained compilation.
    PublishPlans(LocalSavedPlanPublicationRequest),
    /// Admit every saved plan and native target without starting target effects.
    Admit {
        /// Exact retained local compilation consumed by admission.
        compilation: Box<LocalOneShotCompilation>,
        /// Saved-plan, evidence-directory, and materialization-kind selection.
        request: LocalMaterializationAdmissionRequest,
    },
    /// Attempt one admitted run exactly once.
    Settle(Box<LocalMaterializationAdmittedRun>),
    /// Inspect one retained outcome-unknown run without authorizing a retry.
    Reconcile(Box<LocalMaterializationAdmittedRun>),
}

impl std::fmt::Debug for LocalMaterializationCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transition = match self {
            Self::InspectWorkspace { .. } => "InspectWorkspace",
            Self::InspectRepository { .. } => "InspectRepository",
            Self::PublishPlans(_) => "PublishPlans",
            Self::Admit { .. } => "Admit",
            Self::Settle(_) => "Settle",
            Self::Reconcile(_) => "Reconcile",
        };
        formatter
            .debug_tuple("LocalMaterializationCommand")
            .field(&transition)
            .finish()
    }
}

/// Result of one local inspection, planning, or materialization transition.
pub enum LocalMaterializationOutcome {
    /// Workspace comparison plus the unchanged local compilation owner.
    WorkspaceStatus {
        /// Owner required by a later local step.
        compilation: Box<LocalOneShotCompilation>,
        /// Exact SDK result for every selected repository.
        status: LocalWorkspaceStatus,
    },
    /// Repository comparison plus the unchanged local compilation owner.
    RepositoryStatus {
        /// Owner required by a later repository step.
        compilation: Box<LocalOneShotCompilation>,
        /// Complete API-owned status projection for this repository.
        status: LocalRepositoryStatus,
    },
    /// Complete or stopped saved-plan publication batch.
    PlanPublication(LocalSavedPlanPublicationDisposition),
    /// Complete effect-free admission owner for one local run.
    Admitted(Box<LocalMaterializationAdmittedRun>),
    /// Conclusive result or exact rejected settlement with retained custody.
    Settlement(LocalMaterializationSettlementDisposition),
    /// Bounded read-back for one retained outcome-unknown settlement.
    Reconciliation(LocalMaterializationReconciliation),
}

impl std::fmt::Debug for LocalMaterializationOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transition = match self {
            Self::WorkspaceStatus { .. } => "WorkspaceStatus",
            Self::RepositoryStatus { .. } => "RepositoryStatus",
            Self::PlanPublication(_) => "PlanPublication",
            Self::Admitted(_) => "Admitted",
            Self::Settlement(_) => "Settlement",
            Self::Reconciliation(_) => "Reconciliation",
        };
        formatter
            .debug_tuple("LocalMaterializationOutcome")
            .field(&transition)
            .finish()
    }
}

/// Result of attempting one admitted materialization run exactly once.
///
/// Rejection retains the original API error and the same mutated run. The
/// caller may use that run only for a separately selected reconciliation read;
/// the disposition grants no replay or retry permission.
pub enum LocalMaterializationSettlementDisposition {
    /// Every attempted target reached a conclusive recorded result.
    ///
    /// The result can still contain a partial or failed-no-effect target; a
    /// conclusive result does not imply that every repository succeeded.
    Settled(LocalMaterializationRunResult),
    /// The API rejected settlement and returned the mutated run owner.
    Rejected {
        /// Exact API error, without command-layer reclassification.
        error: ApiError,
        /// Same run passed to settlement, retained for possible read-back.
        run: Box<LocalMaterializationAdmittedRun>,
    },
}

impl std::fmt::Debug for LocalMaterializationSettlementDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let disposition = match self {
            Self::Settled(_) => "Settled",
            Self::Rejected { .. } => "Rejected",
        };
        formatter
            .debug_tuple("LocalMaterializationSettlementDisposition")
            .field(&disposition)
            .finish()
    }
}

/// Forward one transition without interpreting the SDK's result or error.
pub(super) fn dispatch(
    api: &PaleApi,
    command: LocalMaterializationCommand,
) -> Result<LocalMaterializationOutcome, ApiError> {
    use LocalMaterializationCommand as Command;
    use LocalMaterializationOutcome as Outcome;

    match command {
        Command::InspectWorkspace { compilation } => api
            .inspect_local_workspace_status(&compilation)
            .map(|status| Outcome::WorkspaceStatus {
                compilation,
                status,
            }),
        Command::InspectRepository {
            compilation,
            repository_id,
        } => api
            .inspect_local_repository_status(&compilation, repository_id)
            .map(|status| Outcome::RepositoryStatus {
                compilation,
                status,
            }),
        Command::PublishPlans(request) => {
            Ok(Outcome::PlanPublication(publish_saved_plans(api, request)))
        }
        Command::Admit {
            compilation,
            request,
        } => api
            .admit_local_materialization_run(&compilation, request)
            .map(Box::new)
            .map(Outcome::Admitted),
        Command::Settle(mut run) => Ok(Outcome::Settlement(
            match api.settle_local_materialization_run(&mut run) {
                Ok(result) => LocalMaterializationSettlementDisposition::Settled(result),
                Err(error) => LocalMaterializationSettlementDisposition::Rejected { error, run },
            },
        )),
        Command::Reconcile(run) => api
            .inspect_local_materialization_reconciliation(&run)
            .map(Outcome::Reconciliation),
    }
}

/// Save plans in the exact repository order retained by the compilation.
fn publish_saved_plans(
    api: &PaleApi,
    request: LocalSavedPlanPublicationRequest,
) -> LocalSavedPlanPublicationDisposition {
    let LocalSavedPlanPublicationRequest {
        compilation,
        kind,
        output_directory,
    } = request;
    let repository_ids = compilation
        .summary()
        .repositories()
        .iter()
        .map(|repository| repository.repository_id().to_owned())
        .collect::<Vec<_>>();
    let mut publications = Vec::with_capacity(repository_ids.len());

    for repository_id in repository_ids {
        let plan = match api.plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new(repository_id.clone(), kind),
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return LocalSavedPlanPublicationDisposition::Stopped {
                    compilation,
                    publications,
                    stopped_repository_id: repository_id,
                    error,
                };
            }
        };
        let counts = plan.counts();
        let publication = match api.publish_local_one_shot_plan(&plan, output_directory.clone()) {
            Ok(publication) => publication,
            Err(error) => {
                return LocalSavedPlanPublicationDisposition::Stopped {
                    compilation,
                    publications,
                    stopped_repository_id: repository_id,
                    error,
                };
            }
        };
        publications.push(LocalSavedPlanPublication {
            counts,
            publication,
        });
    }

    LocalSavedPlanPublicationDisposition::Complete {
        compilation,
        publications,
    }
}
