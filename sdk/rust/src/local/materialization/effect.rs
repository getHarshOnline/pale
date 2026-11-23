//! Saved-plan admission and repository-scoped local effect settlement.

use std::path::{Component, PathBuf};

use pale_application::plan::RunTargetEvidence;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticStageV1, DiagnosticSubjectV1Wire, Outcome, PlanKind, SurfaceExecutableBinding,
};

use super::{git_ignore, LocalOneShotPlan, LocalOneShotPlanRequest};
use crate::error::{ApiError, FailureSpecification};
use crate::{OneShotAdmittedPlan, OneShotCompilation, OneShotPlanKind, PaleApi};

mod settlement;

use settlement::{
    rollback_after_local_effect_failure, settle_target, MutationEvidenceRun, SettledTarget,
};

/// Owner-only Unix mode used for newly published local receipt files.
const RECEIPT_MODE: u32 = 0o600;

/// Local directories and plan class admitted for one complete materialization run.
///
/// Directory spellings are caller-selected ephemeral inputs. They never become
/// semantic repository identities, serialized contracts, or public path handles.
pub struct LocalMaterializationAdmissionRequest {
    kind: OneShotPlanKind,
    saved_plan_directory: String,
    evidence_directory: String,
}

impl std::fmt::Debug for LocalMaterializationAdmissionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalMaterializationAdmissionRequest")
            .field("kind", &self.kind)
            .field(
                "saved_plan_directory_byte_len",
                &self.saved_plan_directory.len(),
            )
            .field(
                "evidence_directory_byte_len",
                &self.evidence_directory.len(),
            )
            .finish()
    }
}

impl LocalMaterializationAdmissionRequest {
    /// Select one run class, exact saved-plan directory, and evidence directory.
    #[must_use]
    pub fn new(
        kind: OneShotPlanKind,
        saved_plan_directory: String,
        evidence_directory: String,
    ) -> Self {
        Self {
            kind,
            saved_plan_directory,
            evidence_directory,
        }
    }
}

/// Repository and path facts returned after one saved plan was published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalOneShotPlanPublication {
    repository_id: String,
    relative_target: String,
    display_target: String,
}

impl LocalOneShotPlanPublication {
    /// Logical repository selected by the compiled plan.
    ///
    /// This identity comes from the plan. It is not inferred from the output
    /// filename or directory.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Filename beneath the caller-selected output directory.
    ///
    /// Join this value to the same admitted output directory to locate the
    /// saved plan. The relative path is not a global file identity.
    #[must_use]
    pub fn relative_target(&self) -> &str {
        &self.relative_target
    }

    /// Local display path returned by the opened filesystem location.
    ///
    /// This value is for presentation. It is not a stable identity or an input
    /// to later admission.
    #[must_use]
    pub fn display_target(&self) -> &str {
        &self.display_target
    }
}

/// Path-free identity of one repository plan admitted before a local run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalMaterializationTargetAdmission {
    repository_id: String,
    plan_digest: String,
}

impl LocalMaterializationTargetAdmission {
    /// Logical repository whose plan was admitted.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Digest of the exact saved plan admitted for this repository.
    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }
}

/// Opaque authority to finish preparing and settle one local materialization run.
///
/// Construction admits every saved plan and machine-local effect without
/// writing. [`PaleApi::settle_local_materialization_run`] verifies the current
/// executable binding, prepares the evidence directory, and admits every receipt slot
/// under one retained lock before starting the first repository effect.
/// Dropping this handle before settlement causes no external effect.
#[must_use = "settle the admitted run or drop it without starting repository effects"]
pub struct LocalMaterializationAdmittedRun {
    kind: OneShotPlanKind,
    targets: Vec<PreparedTarget>,
    target_admissions: Vec<LocalMaterializationTargetAdmission>,
    phase: LocalMaterializationRunPhase,
    compilation: OneShotCompilation,
    surface_executable: Option<SurfaceExecutableBinding>,
}

/// Observation of the exact immutable receipt slot after an unknown outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMaterializationReceiptObservation {
    /// The uncertain operation did not construct receipt bytes before failing.
    NotConstructed,
    /// The exact admitted receipt bytes and owner-only mode are visible.
    Exact,
    /// No receipt is currently visible at the admitted immutable identity.
    Absent,
    /// Different bytes or mode occupy the admitted immutable identity.
    Different,
}

/// Trusted ownership-record relation to the admitted repository transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMaterializationOwnershipObservation {
    /// No trusted ownership record is present.
    Unowned,
    /// The trusted record matches the transition's admitted pre-state.
    MatchesPrior,
    /// The trusted record matches the transition's admitted post-state.
    MatchesPost,
    /// A trusted record exists but matches neither admitted endpoint.
    Other,
}

/// Bounded read-back after settlement reported an unknown outcome.
///
/// This observation does not settle the earlier operation or authorize a retry.
/// It reports only the exact receipt slot and the independently anchored
/// ownership record. Surface and machine-local bytes can still require operator
/// inspection before any retry decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalMaterializationReconciliation {
    repository_id: String,
    plan_digest: String,
    receipt: LocalMaterializationReceiptObservation,
    ownership: LocalMaterializationOwnershipObservation,
}

impl LocalMaterializationReconciliation {
    /// Repository whose unknown settlement is being inspected.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Exact admitted plan identity associated with the unknown settlement.
    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    /// Current observation of the exact immutable receipt slot.
    #[must_use]
    pub const fn receipt(&self) -> LocalMaterializationReceiptObservation {
        self.receipt
    }

    /// Current trusted ownership-record relation to the admitted transition.
    #[must_use]
    pub const fn ownership(&self) -> LocalMaterializationOwnershipObservation {
        self.ownership
    }
}

impl std::fmt::Debug for LocalMaterializationAdmittedRun {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalMaterializationAdmittedRun")
            .field("kind", &self.kind)
            .field("targets", &self.target_admissions)
            .finish_non_exhaustive()
    }
}

impl LocalMaterializationAdmittedRun {
    /// Requested apply, explicit-adoption, or removal operation.
    #[must_use]
    pub const fn kind(&self) -> OneShotPlanKind {
        self.kind
    }

    /// Ordered repository-plan identities admitted before any target effect.
    #[must_use]
    pub fn targets(&self) -> &[LocalMaterializationTargetAdmission] {
        &self.target_admissions
    }
}

/// Whether all, some, or none of the selected repositories completed their plans.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMaterializationRunOutcome {
    /// Every selected repository settled with an immutable success receipt.
    Succeeded,
    /// Earlier repositories succeeded; the next failed without retaining a
    /// materialization effect. Later repositories were not attempted.
    Partial,
    /// The first repository retained no repository or machine-local
    /// materialization effect; its coordination namespace is reported separately.
    FailedNoEffect,
}

/// Recorded outcomes from applying, adopting, or removing local repository files.
///
/// An `Ok` return from [`PaleApi::settle_local_materialization_run`] means this
/// result was recorded, not that every repository succeeded. Check [`Self::outcome`]
/// and the attempted [`Self::targets`]; a failed target stops the remaining run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalMaterializationRunResult {
    kind: OneShotPlanKind,
    outcome: LocalMaterializationRunOutcome,
    target_count: u32,
    succeeded_target_count: u32,
    targets: Vec<LocalMaterializationTargetResult>,
    canonical_bytes: Vec<u8>,
}

/// Outcome of one repository settlement in its project ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalMaterializationTargetOutcome {
    /// The immutable receipt preceded repository and machine-local commit.
    Succeeded,
    /// The admitted repository and machine-local materialization state retained
    /// no effect; persistent coordination state is scoped in the diagnostic.
    FailedNoEffect,
}

/// Presentation-neutral facts for one attempted repository settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalMaterializationTargetResult {
    repository_id: String,
    outcome: LocalMaterializationTargetOutcome,
    changed_surface_count: u32,
    deferred_effect_count: u32,
    local_git_exclude_changed: bool,
    failure: Option<crate::DiagnosticV1>,
}

impl LocalMaterializationTargetResult {
    /// Logical repository attempted by this settlement row.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Proven per-repository settlement outcome.
    #[must_use]
    pub const fn outcome(&self) -> LocalMaterializationTargetOutcome {
        self.outcome
    }

    /// Number of repository surfaces that changed on successful settlement.
    #[must_use]
    pub const fn changed_surface_count(&self) -> u32 {
        self.changed_surface_count
    }

    /// Number of declared effects kept outside this transaction.
    #[must_use]
    pub const fn deferred_effect_count(&self) -> u32 {
        self.deferred_effect_count
    }

    /// Whether successful settlement changed the managed Git-exclude block.
    #[must_use]
    pub const fn local_git_exclude_changed(&self) -> bool {
        self.local_git_exclude_changed
    }

    /// Bounded owner diagnostic for the no-effect materialization result and its
    /// separately observed persistent coordination state.
    #[must_use]
    pub const fn failure(&self) -> Option<&crate::DiagnosticV1> {
        self.failure.as_ref()
    }
}

impl LocalMaterializationRunResult {
    /// Requested apply, explicit-adoption, or removal operation.
    #[must_use]
    pub const fn kind(&self) -> OneShotPlanKind {
        self.kind
    }

    /// Aggregate repository-scoped outcome.
    #[must_use]
    pub const fn outcome(&self) -> LocalMaterializationRunOutcome {
        self.outcome
    }

    /// Number of repository results included before success or the first failure.
    #[must_use]
    pub const fn target_count(&self) -> u32 {
        self.target_count
    }

    /// Number of repositories whose immutable receipts and post-state settled.
    #[must_use]
    pub const fn succeeded_target_count(&self) -> u32 {
        self.succeeded_target_count
    }

    /// Ordered result rows for every attempted repository.
    #[must_use]
    pub fn targets(&self) -> &[LocalMaterializationTargetResult] {
        &self.targets
    }

    /// Canonical JCS bytes of the application-admitted `run-result.v1` record.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

struct PreparedTarget {
    admitted: OneShotAdmittedPlan,
    repository_root: PathBuf,
    lineage_root: PathBuf,
    local_effect: Option<pale_adapter_fs::PreparedMachineLocalFile>,
}

struct UnknownTarget {
    target_index: usize,
    expected_receipt: Option<Vec<u8>>,
}

/// Consuming phase of one local materialization run.
///
/// A run first prepares its evidence directory, then owns the admitted
/// [`MutationEvidenceRun`]. An uncertain native outcome retains both that
/// evidence owner and the exact [`UnknownTarget`] needed for reconciliation.
/// [`Self::Settled`] is terminal: callers cannot reuse earlier preparation or
/// evidence state after a conclusive result.
enum LocalMaterializationRunPhase {
    /// Native preparation completed, but mutation evidence is not yet admitted.
    Prepared {
        /// Confined directory that will own the run's evidence records.
        evidence_directory: String,
    },
    /// Evidence is admitted and the run may attempt bounded native effects.
    EvidenceAdmitted(MutationEvidenceRun),
    /// One attempted target has an outcome that must be reconciled before retry.
    Unknown {
        /// Evidence owner retained across reconciliation.
        evidence: MutationEvidenceRun,
        /// Exact target and optional expected receipt for native read-back.
        target: UnknownTarget,
    },
    /// Every attempted effect has reached a conclusive terminal observation.
    Settled,
}

/// Conservative aggregate observation for auxiliary preparation effects.
///
/// Combining observations preserves the least certain native outcome:
/// [`Self::OutcomeUnknown`] dominates [`Self::Confirmed`], which dominates
/// [`Self::Untouched`]. This prevents a confirmed effect from masking a second
/// effect whose outcome still requires reconciliation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuxiliaryEffectObservation {
    /// No auxiliary native effect was attempted.
    Untouched,
    /// At least one auxiliary effect was confirmed and none is uncertain.
    Confirmed,
    /// At least one auxiliary effect may have occurred without confirmation.
    OutcomeUnknown,
}

impl AuxiliaryEffectObservation {
    /// Return the conservative aggregate of two auxiliary observations.
    const fn combine(self, next: Self) -> Self {
        match (self, next) {
            (Self::OutcomeUnknown, _) | (_, Self::OutcomeUnknown) => Self::OutcomeUnknown,
            (Self::Confirmed, _) | (_, Self::Confirmed) => Self::Confirmed,
            (Self::Untouched, Self::Untouched) => Self::Untouched,
        }
    }
}

impl From<pale_adapter_fs::AdvisoryLockEffect> for AuxiliaryEffectObservation {
    fn from(value: pale_adapter_fs::AdvisoryLockEffect) -> Self {
        match value {
            pale_adapter_fs::AdvisoryLockEffect::Untouched => Self::Untouched,
            pale_adapter_fs::AdvisoryLockEffect::Confirmed => Self::Confirmed,
            pale_adapter_fs::AdvisoryLockEffect::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

struct PreparedDirectory {
    root: pale_adapter_fs::ConfinedRoot,
    observation: AuxiliaryEffectObservation,
}

struct PreparationFailure {
    error: Box<LocalMaterializationError>,
    observation: AuxiliaryEffectObservation,
}

impl PreparationFailure {
    fn new(error: LocalMaterializationError, observation: AuxiliaryEffectObservation) -> Self {
        Self {
            error: Box::new(error),
            observation,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum LocalMaterializationError {
    #[error("{0} directory must be absolute and parent-free")]
    InvalidDirectory(&'static str),
    #[error("{0} directory could not be created")]
    CreateDirectory(&'static str, #[source] std::io::Error),
    #[error("{operation} failed")]
    Filesystem {
        operation: &'static str,
        #[source]
        source: pale_adapter_fs::FsError,
    },
    #[error("saved plan is absent for repository `{0}`")]
    SavedPlanAbsent(String),
    #[error("local Git effect admission failed")]
    GitEffect(#[source] git_ignore::LocalIgnoreError),
    #[error("materialization evidence or result admission failed")]
    Plan(#[source] pale_application::plan::PlanError),
    #[error("materialization contract identity failed")]
    Contract(#[source] pale_contracts::SchemaError),
    #[error("materialization JSON file projection failed")]
    Canonical(#[source] pale_core::canon::CanonError),
    #[error("materialization evidence directory is at capacity")]
    EvidenceCapacity,
    #[error("materialization run invariant failed: {0}")]
    InternalInvariant(&'static str),
    #[error("materialization run is not in the required phase")]
    InvalidPhase,
    #[error("current-executable binding verification failed at the final effect boundary")]
    ExecutableBinding(
        #[source] super::super::executable_identity::CurrentExecutableBindingVerificationError,
    ),
}

impl PaleApi {
    /// Atomically publish one exact complete plan into a confined local directory.
    ///
    /// The output path must be absolute and contain no parent-directory
    /// component. Pale creates the directory when absent, opens it by retained
    /// identity, and verifies it before an owner-only `0600` plan file is
    /// durably replaced.
    /// The target filename is derived from the plan's logical repository and
    /// stored operation class; callers cannot choose a second identity.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for unsafe directory spellings, directory identity
    /// failures, or publication failures. A visibility-uncertain atomic write is
    /// reported as outcome-unknown/reconcile-first.
    pub fn publish_local_one_shot_plan(
        &self,
        plan: &LocalOneShotPlan,
        output_directory: String,
    ) -> Result<LocalOneShotPlanPublication, ApiError> {
        let prepared = open_or_create_directory(&output_directory, "plan-output")
            .map_err(|failure| plan_publication_failure(self, failure))?;
        let root = prepared.root;
        let directory_observation = prepared.observation;
        let suffix = match plan.document.kind() {
            OneShotPlanKind::Apply | OneShotPlanKind::Adopt => "apply",
            OneShotPlanKind::Remove => "remove",
        };
        let repository_id = plan.document.repository_id().to_owned();
        let relative_target = format!("{repository_id}.{suffix}.json");
        let location = root
            .location(&relative_target)
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "resolve saved-plan target",
                source,
            })
            .map_err(|error| {
                plan_publication_failure(
                    self,
                    PreparationFailure::new(error, directory_observation),
                )
            })?;
        let file_bytes = pale_core::canon::pretty_json_document(plan.document.canonical_bytes())
            .map_err(LocalMaterializationError::Canonical)
            .map_err(|error| {
                plan_publication_failure(
                    self,
                    PreparationFailure::new(error, directory_observation),
                )
            })?;
        location
            .write_atomic(&file_bytes, RECEIPT_MODE)
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "publish saved plan",
                source,
            })
            .map_err(|error| {
                let observation = directory_observation.combine(publication_observation(&error));
                plan_publication_failure(self, PreparationFailure::new(error, observation))
            })?;
        Ok(LocalOneShotPlanPublication {
            repository_id,
            relative_target,
            display_target: location.display_path().display().to_string(),
        })
    }

    /// Admit every saved plan and target effect for one local project run.
    ///
    /// Each repository plan is freshly reconstructed from trusted observations,
    /// then compared with its exact confined saved-plan bytes. Machine-local Git
    /// effects are also re-prepared without writing. The evidence-directory
    /// spelling is validated and retained, but no directory or lock is created.
    /// Evidence slots are admitted by [`Self::settle_local_materialization_run`]
    /// before its first repository write.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when a directory spelling is unsafe, a saved plan is
    /// absent or stale, target observation changes, or a Git-local effect no
    /// longer matches. This operation has no external-effect boundary.
    pub fn admit_local_materialization_run(
        &self,
        compilation: &super::super::LocalOneShotCompilation,
        request: LocalMaterializationAdmissionRequest,
    ) -> Result<LocalMaterializationAdmittedRun, ApiError> {
        let LocalMaterializationAdmissionRequest {
            kind,
            saved_plan_directory,
            evidence_directory,
        } = request;
        let saved_root = open_existing_directory(&saved_plan_directory, "saved-plan")
            .map_err(|error| admission_failure(self, error))?;
        let mut targets = Vec::new();
        let mut target_admissions = Vec::new();
        for repository in compilation.summary().repositories() {
            let repository_id = repository.repository_id().to_owned();
            let plan = self.plan_local_one_shot(
                compilation,
                LocalOneShotPlanRequest::new(repository_id.clone(), kind),
            )?;
            let LocalOneShotPlan {
                document,
                repository_root,
                lineage_root,
                git_metadata_root,
                ..
            } = plan;
            let saved = read_saved_plan(&saved_root, &repository_id, kind)
                .map_err(|error| admission_failure(self, error))?;
            let admitted = self.admit_one_shot_saved_plan(document, &saved)?;
            let local_effect = git_ignore::prepare(
                &repository_root,
                git_metadata_root.as_deref(),
                admitted.effect_capability().admitted_plan(),
            )
            .map_err(LocalMaterializationError::GitEffect)
            .map_err(|error| admission_failure(self, error))?;
            target_admissions.push(LocalMaterializationTargetAdmission {
                repository_id,
                plan_digest: admitted.plan_digest().to_owned(),
            });
            targets.push(PreparedTarget {
                admitted,
                repository_root,
                lineage_root,
                local_effect,
            });
        }
        admitted_directory(&evidence_directory, "evidence")
            .map_err(|error| admission_failure(self, error))?;
        Ok(LocalMaterializationAdmittedRun {
            kind,
            targets,
            target_admissions,
            phase: LocalMaterializationRunPhase::Prepared { evidence_directory },
            compilation: compilation.compilation().clone(),
            surface_executable: compilation.host.surface_executable().cloned(),
        })
    }

    /// Apply, adopt, or remove local repository files according to an admitted run.
    ///
    /// This method writes files; it is not a preview. It executes the admitted
    /// repository plans, updates their ownership records and any planned local
    /// Git-exclude protection, and records receipts. Declared deferred effects
    /// remain outside this transaction.
    ///
    /// The run must still be prepared. Before changing repository files, this
    /// method verifies the current executable against the admitted binding,
    /// creates the evidence directory if absent, and checks every receipt slot
    /// and the run-result capacity under one retained lock. Evidence preparation
    /// can itself change directories, lock files, or the prior run summary.
    ///
    /// Repositories are processed in order. For each, file changes are visible
    /// before its exact immutable receipt is published. Publication precedes
    /// commit, which keeps those changes and releases in-process recovery state.
    /// An absent or colliding receipt publication triggers restoration of the
    /// repository and planned machine-local files; coordination lock files and
    /// their parent directories may remain. Uncertain publication instead keeps
    /// the changed files for reconciliation. Recovery does not span a process
    /// crash, and the run is not workspace-wide atomic.
    ///
    /// A target failure stops later repositories without undoing earlier
    /// successes. Check [`LocalMaterializationRunResult::outcome`] even on `Ok`.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for an invalid run phase or failed executable-binding
    /// or evidence checks before repository effects; diagnostics distinguish any evidence
    /// preparation effects. Failed restoration or uncertain target publication
    /// requires reconciliation, not replay. A retained unknown target can be
    /// inspected through [`Self::inspect_local_materialization_reconciliation`].
    ///
    /// Run-summary construction or publication can also fail after target outcomes
    /// are confirmed. That error does not undo those outcomes or justify repeating
    /// repository effects merely to obtain the summary.
    pub fn settle_local_materialization_run(
        &self,
        run: &mut LocalMaterializationAdmittedRun,
    ) -> Result<LocalMaterializationRunResult, ApiError> {
        let evidence_directory = match &run.phase {
            LocalMaterializationRunPhase::Prepared { evidence_directory } => {
                evidence_directory.clone()
            }
            LocalMaterializationRunPhase::EvidenceAdmitted(_)
            | LocalMaterializationRunPhase::Unknown { .. }
            | LocalMaterializationRunPhase::Settled => {
                return Err(reconcile_phase_failure(
                    self,
                    LocalMaterializationError::InvalidPhase,
                ));
            }
        };
        self.verify_bound_current_executable(run.surface_executable.as_ref())
            .map_err(LocalMaterializationError::ExecutableBinding)
            .map_err(|error| pre_effect_failure(self, error))?;
        let prepared = open_or_create_directory(&evidence_directory, "evidence")
            .map_err(|failure| evidence_preparation_failure(self, failure))?;
        let evidence = MutationEvidenceRun::begin(
            prepared.root,
            stored_plan_kind(run.kind),
            run.targets
                .iter()
                .map(|target| target.admitted.effect_capability().admitted_plan()),
            prepared.observation,
        )
        .map_err(|failure| evidence_preparation_failure(self, failure))?;
        run.phase = LocalMaterializationRunPhase::EvidenceAdmitted(evidence);

        let mut target_evidence = Vec::new();
        let mut target_results = Vec::new();
        let mut succeeded = 0_u32;
        for target_index in 0..run.targets.len() {
            let target = &mut run.targets[target_index];
            let effect = target.admitted.effect_capability();
            let transaction = match run.kind {
                OneShotPlanKind::Apply | OneShotPlanKind::Adopt => pale_adapter_fs::apply_effect(
                    &target.repository_root,
                    &target.lineage_root,
                    effect,
                ),
                OneShotPlanKind::Remove => pale_adapter_fs::remove_effect(
                    &target.repository_root,
                    &target.lineage_root,
                    effect,
                ),
            };
            let transaction = match transaction {
                Ok(transaction) => transaction,
                Err(error)
                    if error.coordination_effect()
                        == pale_adapter_fs::AdvisoryLockEffect::OutcomeUnknown
                        && !error.restoration_failed() =>
                {
                    transition_to_unknown(
                        &mut run.phase,
                        UnknownTarget {
                            target_index,
                            expected_receipt: None,
                        },
                    );
                    return Err(coordination_unknown_failure(
                        self,
                        LocalMaterializationError::Filesystem {
                            operation: "prepare repository coordination state",
                            source: error,
                        },
                    ));
                }
                Err(error) if error.restoration_failed() => {
                    transition_to_unknown(
                        &mut run.phase,
                        UnknownTarget {
                            target_index,
                            expected_receipt: None,
                        },
                    );
                    return Err(effect_unknown_failure(
                        self,
                        LocalMaterializationError::Filesystem {
                            operation: "restore failed repository effect",
                            source: error,
                        },
                    ));
                }
                Err(error) => {
                    let repository_id = effect.admitted_plan().plan().repository_id.to_string();
                    let coordination_effect = error.coordination_effect().into();
                    target_evidence.push(RunTargetEvidence::attest_failed_no_effect(
                        effect.admitted_plan().plan().repository_id.clone(),
                    ));
                    target_results.push(failed_target_result(
                        self,
                        repository_id,
                        LocalMaterializationError::Filesystem {
                            operation: "apply repository effect",
                            source: error,
                        },
                        effect.admitted_plan().plan().deferred_effects.len(),
                        coordination_effect,
                    ));
                    break;
                }
            };
            let changed_surface_count = transaction
                .observation()
                .operations
                .iter()
                .filter(|operation| operation.observation.changed())
                .count();
            let local_effect = match target
                .local_effect
                .take()
                .map(pale_adapter_fs::PreparedMachineLocalFile::publish)
            {
                None => None,
                Some(Ok(applied)) => Some(applied),
                Some(Err(error)) => {
                    let repository_id = effect.admitted_plan().plan().repository_id.clone();
                    let coordination_effect = transaction.coordination_effect().into();
                    match rollback_after_local_effect_failure(transaction, error) {
                        Ok(error) => {
                            target_evidence
                                .push(RunTargetEvidence::attest_failed_no_effect(repository_id));
                            target_results.push(failed_target_result(
                                self,
                                effect.admitted_plan().plan().repository_id.to_string(),
                                error,
                                effect.admitted_plan().plan().deferred_effects.len(),
                                coordination_effect,
                            ));
                            break;
                        }
                        Err(error) => {
                            transition_to_unknown(
                                &mut run.phase,
                                UnknownTarget {
                                    target_index,
                                    expected_receipt: None,
                                },
                            );
                            return Err(effect_unknown_failure(self, error));
                        }
                    }
                }
            };
            let local_effect_changed = local_effect
                .as_ref()
                .is_some_and(|effect| effect.observation().observation.changed());
            match settle_target(
                evidence_from_phase(&run.phase),
                effect.admitted_plan(),
                transaction,
                local_effect,
            ) {
                Ok(SettledTarget::Succeeded(evidence)) => {
                    target_evidence.push(evidence);
                    let local_git_exclude_changed = local_effect_changed;
                    target_results.push(LocalMaterializationTargetResult {
                        repository_id: effect.admitted_plan().plan().repository_id.to_string(),
                        outcome: LocalMaterializationTargetOutcome::Succeeded,
                        changed_surface_count: bounded_count(changed_surface_count),
                        deferred_effect_count: bounded_count(
                            effect.admitted_plan().plan().deferred_effects.len(),
                        ),
                        local_git_exclude_changed,
                        failure: None,
                    });
                    succeeded = succeeded.saturating_add(1);
                }
                Ok(SettledTarget::FailedNoEffect(evidence, error, coordination_effect)) => {
                    target_evidence.push(evidence);
                    target_results.push(failed_target_result(
                        self,
                        effect.admitted_plan().plan().repository_id.to_string(),
                        error,
                        effect.admitted_plan().plan().deferred_effects.len(),
                        coordination_effect,
                    ));
                    break;
                }
                Err(failure) => {
                    transition_to_unknown(
                        &mut run.phase,
                        UnknownTarget {
                            target_index,
                            expected_receipt: failure.expected_receipt,
                        },
                    );
                    return Err(effect_unknown_failure(self, *failure.error));
                }
            }
        }
        let result = pale_application::plan::build_run_result(
            run.compilation.application(),
            stored_plan_kind(run.kind),
            target_evidence,
        )
        .map_err(LocalMaterializationError::Plan)
        .map_err(|error| post_effect_failure(self, error))?;
        evidence_from_phase(&run.phase)
            .publish_run_summary(result.canonical_bytes())
            .map_err(|error| post_effect_failure(self, error))?;
        let outcome = match result.outcome() {
            Outcome::Succeeded => LocalMaterializationRunOutcome::Succeeded,
            Outcome::Partial => LocalMaterializationRunOutcome::Partial,
            Outcome::FailedNoEffect => LocalMaterializationRunOutcome::FailedNoEffect,
        };
        let target_count = u32::try_from(result.target_count())
            .map_err(|_| {
                LocalMaterializationError::InternalInvariant(
                    "run-result target count exceeds the API projection width",
                )
            })
            .map_err(|error| post_effect_failure(self, error))?;
        let output = LocalMaterializationRunResult {
            kind: run.kind,
            outcome,
            target_count,
            succeeded_target_count: succeeded,
            targets: target_results,
            canonical_bytes: result.canonical_bytes().to_vec(),
        };
        run.phase = LocalMaterializationRunPhase::Settled;
        Ok(output)
    }

    /// Inspect bounded evidence after settlement returned outcome-unknown.
    ///
    /// The evidence lock remains held by the opaque run while this method reads
    /// the exact immutable receipt slot. Repository ownership is admitted through
    /// its independent lineage anchor and compared only with the saved transition
    /// endpoints. This observation does not settle the operation or authorize a
    /// retry; surface and machine-local bytes can still require operator review.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the run has no unknown outcome, receipt read-back
    /// fails, or trusted ownership cannot be established.
    pub fn inspect_local_materialization_reconciliation(
        &self,
        run: &LocalMaterializationAdmittedRun,
    ) -> Result<LocalMaterializationReconciliation, ApiError> {
        let (evidence, unknown) = match &run.phase {
            LocalMaterializationRunPhase::Unknown { evidence, target } => (evidence, target),
            LocalMaterializationRunPhase::Prepared { .. }
            | LocalMaterializationRunPhase::EvidenceAdmitted(_)
            | LocalMaterializationRunPhase::Settled => {
                return Err(reconcile_phase_failure(
                    self,
                    LocalMaterializationError::InvalidPhase,
                ));
            }
        };
        let target = run.targets.get(unknown.target_index).ok_or_else(|| {
            reconcile_phase_failure(
                self,
                LocalMaterializationError::InternalInvariant(
                    "unknown target is absent from the admitted run",
                ),
            )
        })?;
        let admitted = target.admitted.effect_capability().admitted_plan();
        let receipt = match unknown.expected_receipt.as_deref() {
            None => LocalMaterializationReceiptObservation::NotConstructed,
            Some(expected) => evidence
                .observe_receipt(admitted, expected)
                .map_err(|error| reconciliation_read_failure(self, error))?,
        };
        let contract = pale_application::plan::materialization_contract_identity(
            run.compilation.application(),
        )
        .map_err(LocalMaterializationError::Contract)
        .map_err(|error| reconciliation_read_failure(self, error))?;
        let current = pale_adapter_fs::read_materialization_state(
            &target.repository_root,
            &target.lineage_root,
            &admitted.plan().repository_id,
            &contract,
        )
        .map_err(|source| LocalMaterializationError::Filesystem {
            operation: "read reconciled ownership state",
            source,
        })
        .map_err(|error| reconciliation_read_failure(self, error))?;
        Ok(LocalMaterializationReconciliation {
            repository_id: admitted.plan().repository_id.to_string(),
            plan_digest: admitted.digest().to_string(),
            receipt,
            ownership: reconcile_ownership(admitted.plan(), current.as_ref()),
        })
    }
}

fn open_existing_directory(
    value: &str,
    label: &'static str,
) -> Result<pale_adapter_fs::ConfinedRoot, LocalMaterializationError> {
    let path = admitted_directory(value, label)?;
    pale_adapter_fs::ConfinedRoot::open(&path).map_err(|source| {
        LocalMaterializationError::Filesystem {
            operation: "open local directory",
            source,
        }
    })
}

fn open_or_create_directory(
    value: &str,
    label: &'static str,
) -> Result<PreparedDirectory, PreparationFailure> {
    let path = admitted_directory(value, label)
        .map_err(|error| PreparationFailure::new(error, AuxiliaryEffectObservation::Untouched))?;
    let observation = match std::fs::symlink_metadata(&path) {
        Ok(_) => AuxiliaryEffectObservation::Untouched,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(&path).map_err(|source| {
                PreparationFailure::new(
                    LocalMaterializationError::CreateDirectory(label, source),
                    AuxiliaryEffectObservation::OutcomeUnknown,
                )
            })?;
            AuxiliaryEffectObservation::Confirmed
        }
        Err(source) => {
            return Err(PreparationFailure::new(
                LocalMaterializationError::CreateDirectory(label, source),
                AuxiliaryEffectObservation::Untouched,
            ));
        }
    };
    let root = pale_adapter_fs::ConfinedRoot::open(&path)
        .map_err(|source| LocalMaterializationError::Filesystem {
            operation: "open local directory",
            source,
        })
        .map_err(|error| PreparationFailure::new(error, observation))?;
    root.verify_path_identity()
        .map_err(|source| LocalMaterializationError::Filesystem {
            operation: "verify local directory identity",
            source,
        })
        .map_err(|error| PreparationFailure::new(error, observation))?;
    Ok(PreparedDirectory { root, observation })
}

fn admitted_directory(
    value: &str,
    label: &'static str,
) -> Result<PathBuf, LocalMaterializationError> {
    let path = PathBuf::from(value);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(LocalMaterializationError::InvalidDirectory(label));
    }
    Ok(path)
}

fn read_saved_plan(
    root: &pale_adapter_fs::ConfinedRoot,
    repository_id: &str,
    kind: OneShotPlanKind,
) -> Result<Vec<u8>, LocalMaterializationError> {
    let relative = format!("{repository_id}.{}.json", stored_plan_kind(kind));
    root.location(&relative)
        .and_then(|location| {
            location.read_file(pale_adapter_fs::ReadLimit::new(
                pale_application::plan::SAVED_PLAN_BYTE_LIMIT,
            ))
        })
        .map_err(|source| LocalMaterializationError::Filesystem {
            operation: "read saved plan",
            source,
        })?
        .map(|file| file.bytes)
        .ok_or_else(|| LocalMaterializationError::SavedPlanAbsent(repository_id.to_owned()))
}

const fn stored_plan_kind(kind: OneShotPlanKind) -> PlanKind {
    match kind {
        OneShotPlanKind::Apply | OneShotPlanKind::Adopt => PlanKind::Apply,
        OneShotPlanKind::Remove => PlanKind::Remove,
    }
}

fn reconcile_ownership(
    plan: &pale_core::records::AgentSurfacePlanV1,
    current: Option<&pale_core::records::MaterializationStateV1>,
) -> LocalMaterializationOwnershipObservation {
    let Some(current) = current else {
        return LocalMaterializationOwnershipObservation::Unowned;
    };
    if plan
        .transition
        .prior()
        .is_some_and(|prior| prior.state_digest == current.state_digest)
    {
        LocalMaterializationOwnershipObservation::MatchesPrior
    } else if plan
        .transition
        .next_state()
        .is_some_and(|next| next.state_digest == current.state_digest)
    {
        LocalMaterializationOwnershipObservation::MatchesPost
    } else {
        LocalMaterializationOwnershipObservation::Other
    }
}

fn evidence_from_phase(phase: &LocalMaterializationRunPhase) -> &MutationEvidenceRun {
    match phase {
        LocalMaterializationRunPhase::EvidenceAdmitted(evidence) => evidence,
        LocalMaterializationRunPhase::Prepared { .. }
        | LocalMaterializationRunPhase::Unknown { .. }
        | LocalMaterializationRunPhase::Settled => {
            unreachable!("repository effects require admitted evidence")
        }
    }
}

fn transition_to_unknown(phase: &mut LocalMaterializationRunPhase, target: UnknownTarget) {
    let admitted = std::mem::replace(phase, LocalMaterializationRunPhase::Settled);
    *phase = match admitted {
        LocalMaterializationRunPhase::EvidenceAdmitted(evidence) => {
            LocalMaterializationRunPhase::Unknown { evidence, target }
        }
        LocalMaterializationRunPhase::Prepared { .. }
        | LocalMaterializationRunPhase::Unknown { .. }
        | LocalMaterializationRunPhase::Settled => {
            unreachable!("unknown target outcome follows evidence admission")
        }
    };
}

fn bounded_count(count: usize) -> u32 {
    u32::try_from(count).expect("materialization counts are bounded by admitted plans")
}

fn failed_target_result(
    api: &PaleApi,
    repository_id: String,
    error: LocalMaterializationError,
    deferred_effect_count: usize,
    coordination_effect: AuxiliaryEffectObservation,
) -> LocalMaterializationTargetResult {
    let subject_id = repository_id.clone();
    let diagnostic = failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::OperationalFailure,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::Confirmed,
            code: "local_materialization_target_failed_no_effect",
            summary: "the admitted repository and machine-local materialization state has no retained effect",
            help: Vec::new(),
            subject: Some(diagnostic_subject("repository", &subject_id)),
            facts: vec![
                diagnostic_fact(
                    "effect_scope",
                    "repository_machine_local_materialization_and_coordination",
                ),
                diagnostic_fact("repository_materialization_effect", "failed_no_effect"),
                diagnostic_fact("machine_local_materialization_effect", "failed_no_effect"),
                diagnostic_fact(
                    "coordination_effect",
                    auxiliary_effect_label(coordination_effect),
                ),
            ],
        },
    )
    .into_diagnostic();
    LocalMaterializationTargetResult {
        repository_id,
        outcome: LocalMaterializationTargetOutcome::FailedNoEffect,
        changed_surface_count: 0,
        deferred_effect_count: bounded_count(deferred_effect_count),
        local_git_exclude_changed: false,
        failure: Some(diagnostic),
    }
}

fn admission_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    let (disposition, retry) = admission_policy(&error);
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            code: "local_materialization_admission_failed",
            summary: "local materialization plans and target effects could not be admitted",
            help: Vec::new(),
            subject: None,
            facts: Vec::new(),
        },
    )
}

fn plan_publication_failure(api: &PaleApi, preparation: PreparationFailure) -> ApiError {
    let PreparationFailure { error, observation } = preparation;
    let target_visibility_unknown = matches!(
        &*error,
        LocalMaterializationError::Filesystem { source, .. }
            if source.effect_visibility() == pale_adapter_fs::EffectVisibility::MayBeVisible
    );
    let (disposition, retry, effect, help) = match observation {
        AuxiliaryEffectObservation::OutcomeUnknown => (
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
            if target_visibility_unknown {
                vec![
                    "Inspect the exact saved-plan target and its output directory before deciding whether to retry.",
                ]
            } else {
                vec![
                    "Inspect output-directory coordination state before retrying; no saved-plan target publication was reported visible.",
                ]
            },
        ),
        AuxiliaryEffectObservation::Confirmed => {
            let (disposition, retry) = admission_policy(&error);
            (
                disposition,
                retry,
                DiagnosticEffectCertaintyV1::Confirmed,
                vec![
                    "An output-directory coordination effect is confirmed; no saved-plan target publication was reported visible.",
                ],
            )
        }
        AuxiliaryEffectObservation::Untouched => {
            let (disposition, retry) = admission_policy(&error);
            (
                disposition,
                retry,
                DiagnosticEffectCertaintyV1::NotStarted,
                vec!["No output-directory or saved-plan target effect was reported visible."],
            )
        }
    };
    failure(
        api,
        *error,
        LocalMaterializationFailurePolicy {
            disposition,
            retry,
            effect,
            code: "local_plan_publication_failed",
            summary: "the complete local plan could not be published atomically",
            help,
            subject: Some(diagnostic_subject(
                "operation",
                "local_one_shot_plan_publication",
            )),
            facts: vec![
                diagnostic_fact(
                    "effect_scope",
                    if target_visibility_unknown {
                        "saved_plan_target_and_output_coordination"
                    } else {
                        "output_coordination"
                    },
                ),
                diagnostic_fact(
                    "output_coordination_effect",
                    auxiliary_effect_label(observation),
                ),
                diagnostic_fact(
                    "saved_plan_target_effect",
                    if target_visibility_unknown {
                        "outcome_unknown"
                    } else {
                        "not_started"
                    },
                ),
            ],
        },
    )
}

fn publication_observation(error: &LocalMaterializationError) -> AuxiliaryEffectObservation {
    match error {
        LocalMaterializationError::Filesystem { source, .. }
            if source.effect_visibility() == pale_adapter_fs::EffectVisibility::MayBeVisible
                || matches!(source, pale_adapter_fs::FsError::CleanupFailed { .. }) =>
        {
            AuxiliaryEffectObservation::OutcomeUnknown
        }
        _ => AuxiliaryEffectObservation::Untouched,
    }
}

fn admission_policy(
    error: &LocalMaterializationError,
) -> (DiagnosticDispositionV1, DiagnosticRetryV1) {
    use DiagnosticDispositionV1::{Blocked, InternalFailure, InvalidInput, OperationalFailure};
    use DiagnosticRetryV1::{AfterCorrection, AfterStateChange, NotIndicated};
    match error {
        LocalMaterializationError::InvalidDirectory(_)
        | LocalMaterializationError::SavedPlanAbsent(_) => (InvalidInput, AfterCorrection),
        LocalMaterializationError::GitEffect(error) => match error {
            git_ignore::LocalIgnoreError::Invalid(_)
            | git_ignore::LocalIgnoreError::InvalidTarget
            | git_ignore::LocalIgnoreError::Utf8(_)
            | git_ignore::LocalIgnoreError::GitMetadataUtf8(_) => (Blocked, AfterStateChange),
            git_ignore::LocalIgnoreError::Filesystem(_)
            | git_ignore::LocalIgnoreError::Source(_)
            | git_ignore::LocalIgnoreError::Io(_) => (OperationalFailure, NotIndicated),
        },
        LocalMaterializationError::Filesystem { source, .. } => fs_admission_policy(source),
        LocalMaterializationError::EvidenceCapacity => (Blocked, AfterStateChange),
        LocalMaterializationError::InvalidPhase
        | LocalMaterializationError::ExecutableBinding(_) => (Blocked, AfterStateChange),
        LocalMaterializationError::CreateDirectory(_, _) => (OperationalFailure, NotIndicated),
        LocalMaterializationError::Plan(_)
        | LocalMaterializationError::Contract(_)
        | LocalMaterializationError::Canonical(_)
        | LocalMaterializationError::InternalInvariant(_) => (InternalFailure, NotIndicated),
    }
}

fn fs_admission_policy(
    error: &pale_adapter_fs::FsError,
) -> (DiagnosticDispositionV1, DiagnosticRetryV1) {
    use DiagnosticDispositionV1::{Blocked, InvalidInput, OperationalFailure};
    use DiagnosticRetryV1::{AfterCorrection, AfterStateChange, NotIndicated};
    match error {
        pale_adapter_fs::FsError::FileReadLimit { .. }
        | pale_adapter_fs::FsError::RecordSizeLimit { .. }
        | pale_adapter_fs::FsError::DirectoryEntryLimit { .. }
        | pale_adapter_fs::FsError::TraversalEntryLimit { .. }
        | pale_adapter_fs::FsError::TraversalDepthLimit { .. }
        | pale_adapter_fs::FsError::InvalidMode { .. }
        | pale_adapter_fs::FsError::DuplicateRequestPath { .. }
        | pale_adapter_fs::FsError::Escape(_)
        | pale_adapter_fs::FsError::MissingContent(_)
        | pale_adapter_fs::FsError::WrongPlanKind { .. } => (InvalidInput, AfterCorrection),
        pale_adapter_fs::FsError::Invariant { .. }
        | pale_adapter_fs::FsError::Collision(..)
        | pale_adapter_fs::FsError::StageMismatch(_)
        | pale_adapter_fs::FsError::InvalidState(_)
        | pale_adapter_fs::FsError::InvalidMachineLocalEffect(_)
        | pale_adapter_fs::FsError::UnexpectedExpiringRecordEntry { .. } => {
            (Blocked, AfterStateChange)
        }
        pale_adapter_fs::FsError::Io { .. }
        | pale_adapter_fs::FsError::Data { .. }
        | pale_adapter_fs::FsError::Timeout { .. }
        | pale_adapter_fs::FsError::UnsupportedPlatform { .. }
        | pale_adapter_fs::FsError::CleanupFailed { .. }
        | pale_adapter_fs::FsError::EffectUncertain { .. }
        | pale_adapter_fs::FsError::RestoreFailed { .. }
        | pale_adapter_fs::FsError::RecoveryFailed { .. }
        | pale_adapter_fs::FsError::BatchFailed { .. } => (OperationalFailure, NotIndicated),
        pale_adapter_fs::FsError::Coordination { source, .. } => fs_admission_policy(source),
    }
}

fn evidence_preparation_failure(api: &PaleApi, preparation: PreparationFailure) -> ApiError {
    let PreparationFailure { error, observation } = preparation;
    let (disposition, retry, effect, help) = match observation {
        AuxiliaryEffectObservation::OutcomeUnknown => (
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
            vec![
                "Inspect the evidence directory, coordination lock, and prior run-result slot before retrying; no repository target was started.",
            ],
        ),
        AuxiliaryEffectObservation::Confirmed => {
            let (disposition, retry) = admission_policy(&error);
            (
                disposition,
                retry,
                DiagnosticEffectCertaintyV1::Confirmed,
                vec![
                    "A completed evidence-directory, coordination-lock, or prior-result invalidation effect is confirmed; no repository target was started.",
                ],
            )
        }
        AuxiliaryEffectObservation::Untouched => {
            let (disposition, retry) = admission_policy(&error);
            (
                disposition,
                retry,
                DiagnosticEffectCertaintyV1::NotStarted,
                vec!["No evidence-side or repository target effect was started."],
            )
        }
    };
    failure(
        api,
        *error,
        LocalMaterializationFailurePolicy {
            disposition,
            retry,
            effect,
            code: "local_materialization_evidence_admission_failed",
            summary: "evidence preparation failed before any repository target effect",
            help,
            subject: Some(diagnostic_subject(
                "operation",
                "local_materialization_evidence_preparation",
            )),
            facts: vec![
                diagnostic_fact("effect_scope", "evidence_preparation"),
                diagnostic_fact(
                    "evidence_auxiliary_effect",
                    auxiliary_effect_label(observation),
                ),
                diagnostic_fact("repository_effect", "not_started"),
            ],
        },
    )
}

fn reconcile_phase_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::Blocked,
            retry: DiagnosticRetryV1::AfterStateChange,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            code: "local_materialization_phase_rejected",
            summary: "the materialization handle is not in the required phase",
            help: Vec::new(),
            subject: None,
            facts: Vec::new(),
        },
    )
}

fn reconciliation_read_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
            retry: DiagnosticRetryV1::ReconcileFirst,
            effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
            code: "local_materialization_reconciliation_incomplete",
            summary: "the unknown materialization outcome could not be read back safely",
            help: vec!["Do not repeat the earlier operation while its outcome remains unknown."],
            subject: None,
            facts: Vec::new(),
        },
    )
}

fn pre_effect_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::Blocked,
            retry: DiagnosticRetryV1::AfterStateChange,
            effect: DiagnosticEffectCertaintyV1::NotStarted,
            code: "local_materialization_runtime_rejected",
            summary: "the current executable did not satisfy its admitted binding",
            help: Vec::new(),
            subject: None,
            facts: Vec::new(),
        },
    )
}

fn effect_unknown_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    let restoration_failed = matches!(
        &error,
        LocalMaterializationError::Filesystem { source, .. } if source.restoration_failed()
    );
    let coordination_effect = match &error {
        LocalMaterializationError::Filesystem { source, .. } => {
            AuxiliaryEffectObservation::from(source.coordination_effect())
        }
        _ => AuxiliaryEffectObservation::Untouched,
    };
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
            retry: DiagnosticRetryV1::ReconcileFirst,
            effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
            code: "local_materialization_outcome_unknown",
            summary: "local materialization state requires explicit reconciliation",
            help: vec![
                "Do not repeat this operation until retained receipts and target state are reconciled.",
            ],
            subject: Some(diagnostic_subject(
                "operation",
                "local_materialization_reconciliation",
            )),
            facts: if restoration_failed {
                vec![
                    diagnostic_fact(
                        "effect_scope",
                        "repository_machine_local_materialization_and_coordination",
                    ),
                    diagnostic_fact("repository_materialization_effect", "outcome_unknown"),
                    diagnostic_fact("machine_local_materialization_effect", "outcome_unknown"),
                    diagnostic_fact(
                        "coordination_effect",
                        auxiliary_effect_label(coordination_effect),
                    ),
                ]
            } else {
                vec![
                    diagnostic_fact(
                        "effect_scope",
                        "immutable_receipt_and_retained_materialization_post_state",
                    ),
                    diagnostic_fact("immutable_receipt_effect", "outcome_unknown"),
                    diagnostic_fact("repository_materialization_effect", "confirmed_post_state"),
                    diagnostic_fact(
                        "coordination_effect",
                        auxiliary_effect_label(coordination_effect),
                    ),
                ]
            },
        },
    )
}

fn coordination_unknown_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
            retry: DiagnosticRetryV1::ReconcileFirst,
            effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
            code: "local_materialization_coordination_outcome_unknown",
            summary: "persistent repository coordination state is unknown; materialization did not start",
            help: vec![
                "Inspect the repository .pale/apply.lock namespace and its parent before deciding whether to retry.",
            ],
            subject: Some(diagnostic_subject(
                "operation",
                "local_materialization_repository_coordination",
            )),
            facts: vec![
                diagnostic_fact("effect_scope", "repository_coordination"),
                diagnostic_fact("coordination_effect", "outcome_unknown"),
                diagnostic_fact("repository_materialization_effect", "not_started"),
                diagnostic_fact("machine_local_materialization_effect", "not_started"),
            ],
        },
    )
}

fn post_effect_failure(api: &PaleApi, error: LocalMaterializationError) -> ApiError {
    failure(
        api,
        error,
        LocalMaterializationFailurePolicy {
            disposition: DiagnosticDispositionV1::OperationalFailure,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::Confirmed,
            code: "local_materialization_run_result_failed",
            summary: "the confirmed repository outcomes could not be published as a run result",
            help: vec![
                "Do not repeat repository effects solely because run-result publication failed.",
            ],
            subject: None,
            facts: Vec::new(),
        },
    )
}

struct LocalMaterializationFailurePolicy {
    disposition: DiagnosticDispositionV1,
    retry: DiagnosticRetryV1,
    effect: DiagnosticEffectCertaintyV1,
    code: &'static str,
    summary: &'static str,
    help: Vec<&'static str>,
    subject: Option<DiagnosticSubjectV1Wire>,
    facts: Vec<DiagnosticFactV1Wire>,
}

fn failure(
    api: &PaleApi,
    error: LocalMaterializationError,
    policy: LocalMaterializationFailurePolicy,
) -> ApiError {
    ApiError::new_with_opaque_source(
        &api.diagnostic_identity,
        error,
        FailureSpecification {
            code: policy.code,
            stage: DiagnosticStageV1::Materialization,
            disposition: policy.disposition,
            summary: policy.summary,
            subject: policy.subject,
            schema_coordinate: None,
            location: None,
            retry: policy.retry,
            effect: policy.effect,
            request: None,
            notes: Vec::new(),
            help: policy.help,
            facts: policy.facts,
        },
    )
}

const fn auxiliary_effect_label(observation: AuxiliaryEffectObservation) -> &'static str {
    match observation {
        AuxiliaryEffectObservation::Untouched => "not_started",
        AuxiliaryEffectObservation::Confirmed => "confirmed",
        AuxiliaryEffectObservation::OutcomeUnknown => "outcome_unknown",
    }
}

fn diagnostic_subject(kind: &str, id: &str) -> DiagnosticSubjectV1Wire {
    DiagnosticSubjectV1Wire {
        kind: kind.to_owned(),
        id: id.to_owned(),
    }
}

fn diagnostic_fact(key: &str, value: &str) -> DiagnosticFactV1Wire {
    DiagnosticFactV1Wire {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/local_materialization_effect.rs"]
mod tests;
