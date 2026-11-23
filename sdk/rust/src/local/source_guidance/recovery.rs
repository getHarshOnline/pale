//! Owned recovery review and fresh admitted native execution through the SDK.

use super::super::LocalOneShotCompilationRequest;
use super::{
    effect::{
        admission_failure, native_failure, reviewed_digest, LocalSourceGuidanceCoordinationEffect,
        NativeBoundary,
    },
    source_context, LocalSourceGuidanceFile,
};
use crate::{ApiError, EffectiveProfileCompilation, PaleApi, SourceGuidanceHost};
use pale_adapter_fs as native;
use pale_core::records::LogicalRepositoryIdV1;

/// Explicit reviewed recovery choice; neither choice replays application work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSourceGuidanceRecoveryIntent {
    /// Restore only retained before-images, preserving independent changes.
    RestoreBefore,
    /// Keep a complete witnessed after-state and finish commit/cleanup.
    FinalizePublished,
}
impl LocalSourceGuidanceRecoveryIntent {
    fn native(self) -> native::SourceGuidanceRecoveryIntent {
        match self {
            Self::RestoreBefore => native::SourceGuidanceRecoveryIntent::RestoreBefore,
            Self::FinalizePublished => native::SourceGuidanceRecoveryIntent::FinalizePublished,
        }
    }
}

/// Exact source selection and native recovery intent; construction performs no I/O.
#[derive(Debug)]
pub struct LocalSourceGuidanceRecoveryRequest {
    selection: LocalOneShotCompilationRequest,
    repository_id: String,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceRecoveryIntent,
}
impl LocalSourceGuidanceRecoveryRequest {
    /// Select one logical repository and host without inferring a private binding.
    #[must_use]
    pub fn new(
        selection: LocalOneShotCompilationRequest,
        repository_id: String,
        host: SourceGuidanceHost,
        intent: LocalSourceGuidanceRecoveryIntent,
    ) -> Self {
        Self {
            selection,
            repository_id,
            host,
            intent,
        }
    }
}

/// Recovery evidence is distinct from inference about who executed a file write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSourceGuidanceRecoveryEvidence {
    /// Matching before-image bytes/mode or matching absence; preserve the entry.
    BeforeImagePresent,
    /// Exact checkpointed after-image bytes, mode and native identity.
    CheckpointedAfterImage,
    /// Intended after-image bytes/mode match, with no recorded post identity.
    UncheckpointedAfterImage,
}
impl LocalSourceGuidanceRecoveryEvidence {
    fn from_native(value: native::SourceGuidanceRecoveryEvidence) -> Self {
        match value {
            native::SourceGuidanceRecoveryEvidence::BeforeImagePresent => Self::BeforeImagePresent,
            native::SourceGuidanceRecoveryEvidence::CheckpointedAfterImage => {
                Self::CheckpointedAfterImage
            }
            native::SourceGuidanceRecoveryEvidence::UncheckpointedAfterImage => {
                Self::UncheckpointedAfterImage
            }
        }
    }
}

/// File review data reuses the normal baseline/change model and adds native evidence.
#[derive(serde::Serialize)]
pub struct LocalSourceGuidanceRecoveryFile {
    #[serde(flatten)]
    file: LocalSourceGuidanceFile,
    evidence: LocalSourceGuidanceRecoveryEvidence,
}
impl LocalSourceGuidanceRecoveryFile {
    /// Exact current baseline and proposed complete bytes, including local facts.
    #[must_use]
    pub const fn file(&self) -> &LocalSourceGuidanceFile {
        &self.file
    }
    /// Checkpointed, uncheckpointed or matching-before-image evidence.
    #[must_use]
    pub const fn evidence(&self) -> LocalSourceGuidanceRecoveryEvidence {
        self.evidence
    }
}

/// Directory cleanup is conditional and preserves independent contents/replacements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSourceGuidanceRecoveryDirectoryState {
    /// Already absent; nothing to create or remove.
    Missing,
    /// The recorded identity matches; remove only if still empty.
    IdentityMatched,
    /// Independent replacement; retain it, including when empty.
    Replaced,
}
/// One exact directory considered for conditional native cleanup.
#[derive(Debug, serde::Serialize)]
pub struct LocalSourceGuidanceRecoveryDirectory {
    rel_path: String,
    state: LocalSourceGuidanceRecoveryDirectoryState,
}
impl LocalSourceGuidanceRecoveryDirectory {
    /// Repository-relative directory, without an absolute native path.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.rel_path
    }
    /// Exact identity state; emptiness is checked atomically at removal.
    #[must_use]
    pub const fn state(&self) -> LocalSourceGuidanceRecoveryDirectoryState {
        self.state
    }
}

/// Explicit local recovery report; full proposed bytes may contain project facts.
#[derive(serde::Serialize)]
pub struct LocalSourceGuidanceRecoveryPlanReport {
    repository_id: String,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceRecoveryIntent,
    digest: String,
    files: Vec<LocalSourceGuidanceRecoveryFile>,
    directories: Vec<LocalSourceGuidanceRecoveryDirectory>,
}
impl LocalSourceGuidanceRecoveryPlanReport {
    /// Selected logical repository.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    /// Selected source-guidance host layout.
    #[must_use]
    pub const fn host(&self) -> SourceGuidanceHost {
        self.host
    }
    /// Decision included in the exact reviewed identity.
    #[must_use]
    pub const fn intent(&self) -> LocalSourceGuidanceRecoveryIntent {
        self.intent
    }
    /// Identity of the journal, native baselines and complete recovery decision.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }
    /// Complete file actions with explicit uncertainty; retain within project scope.
    #[must_use]
    pub fn files(&self) -> &[LocalSourceGuidanceRecoveryFile] {
        &self.files
    }
    /// Conditional identity-bound directory cleanup.
    #[must_use]
    pub fn directories(&self) -> &[LocalSourceGuidanceRecoveryDirectory] {
        &self.directories
    }
}

/// Opaque captured review, not execution authority and never accepted as saved input.
pub struct LocalSourceGuidanceRecoveryPlan {
    compilation: EffectiveProfileCompilation,
    repository: LogicalRepositoryIdV1,
    native: native::SourceGuidanceRecoveryPlan,
    report: LocalSourceGuidanceRecoveryPlanReport,
}
impl std::fmt::Debug for LocalSourceGuidanceRecoveryPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSourceGuidanceRecoveryPlan")
            .field("repository_id", &self.report.repository_id)
            .field("host", &self.report.host)
            .field("intent", &self.report.intent)
            .field("digest", &self.report.digest)
            .field("files", &self.report.files.len())
            .finish_non_exhaustive()
    }
}
impl LocalSourceGuidanceRecoveryPlan {
    /// SDK-owned deliberately serializable local report; raw native handles stay private.
    #[must_use]
    pub const fn report(&self) -> &LocalSourceGuidanceRecoveryPlanReport {
        &self.report
    }
}

/// Fresh selection and exact reviewed recovery identity, rather than a cached handle.
pub struct LocalSourceGuidanceRecoveryExecutionRequest {
    selection: LocalSourceGuidanceRecoveryRequest,
    reviewed_digest: String,
}
impl std::fmt::Debug for LocalSourceGuidanceRecoveryExecutionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSourceGuidanceRecoveryExecutionRequest")
            .field("selection", &self.selection)
            .field("reviewed_digest_byte_len", &self.reviewed_digest.len())
            .finish()
    }
}
impl LocalSourceGuidanceRecoveryExecutionRequest {
    /// Bind explicit restore/finalize intent to its reviewed identity.
    #[must_use]
    pub fn new(selection: LocalSourceGuidanceRecoveryRequest, reviewed_digest: String) -> Self {
        Self {
            selection,
            reviewed_digest,
        }
    }
}

/// Confirmed native reconciliation, not coding-host use or runtime activation.
#[derive(Debug, serde::Serialize)]
pub struct LocalSourceGuidanceRecoveryResult {
    repository_id: String,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceRecoveryIntent,
    recovery_digest: String,
    changed_files: usize,
    removed_directories: Vec<String>,
    retained_directories: Vec<String>,
    coordination_effect: LocalSourceGuidanceCoordinationEffect,
}
impl LocalSourceGuidanceRecoveryResult {
    /// Selected logical repository.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    /// Selected native layout.
    #[must_use]
    pub const fn host(&self) -> SourceGuidanceHost {
        self.host
    }
    /// Decision confirmed by native reconciliation.
    #[must_use]
    pub const fn intent(&self) -> LocalSourceGuidanceRecoveryIntent {
        self.intent
    }
    /// Exact reviewed recovery actually rebuilt and admitted.
    #[must_use]
    pub fn recovery_digest(&self) -> &str {
        &self.recovery_digest
    }
    /// Changed guidance files, excluding retained matching entries.
    #[must_use]
    pub const fn changed_files(&self) -> usize {
        self.changed_files
    }
    /// Recorded empty directories removed by exact native identity.
    #[must_use]
    pub fn removed_directories(&self) -> &[String] {
        &self.removed_directories
    }
    /// Preserved replacement or nonempty directories.
    #[must_use]
    pub fn retained_directories(&self) -> &[String] {
        &self.retained_directories
    }
    /// Persistent coordination effect, independent of file restoration.
    #[must_use]
    pub const fn coordination_effect(&self) -> LocalSourceGuidanceCoordinationEffect {
        self.coordination_effect
    }
}

impl PaleApi {
    /// Capture native recovery and validate logical scope before exposing its review.
    ///
    /// `None` establishes only that the selected root had no pending journal at
    /// observation time. This operation creates no lock, file or effect capability.
    ///
    /// # Errors
    /// Rejects invalid source context, malformed/unsafe recovery state, independent
    /// changes or another logical owner/host. Native bodies and paths stay opaque
    /// in errors; intentional report serialization includes local before-images.
    pub fn plan_local_source_guidance_recovery(
        &self,
        request: LocalSourceGuidanceRecoveryRequest,
    ) -> Result<Option<LocalSourceGuidanceRecoveryPlan>, ApiError> {
        let LocalSourceGuidanceRecoveryRequest {
            selection,
            repository_id,
            host,
            intent,
        } = request;
        let (compilation, repository, root) = source_context(self, selection, &repository_id)?;
        let Some(plan) =
            native::plan_source_guidance_recovery(root, intent.native()).map_err(|error| {
                native_failure(
                    self,
                    error,
                    NativeBoundary::Inspection,
                    "source_guidance_recovery_observation_failed",
                )
            })?
        else {
            return Ok(None);
        };
        compilation
            .compilation
            .validate_source_guidance_recovery_scope(
                &repository,
                host.application(),
                plan.proposal(),
            )
            .map_err(|error| {
                let d = error.failure_disposition();
                admission_failure(
                    self,
                    error,
                    d,
                    "source_guidance_recovery_scope_rejected",
                    "native recovery does not match the selected logical owner, host or path scope",
                )
            })?;
        let files = plan
            .files()
            .iter()
            .map(|file| LocalSourceGuidanceRecoveryFile {
                file: LocalSourceGuidanceFile::from_native(
                    file.rel_path(),
                    file.baseline(),
                    file.change(),
                ),
                evidence: LocalSourceGuidanceRecoveryEvidence::from_native(file.evidence()),
            })
            .collect();
        let directories = plan
            .directories()
            .iter()
            .map(|directory| LocalSourceGuidanceRecoveryDirectory {
                rel_path: directory.rel_path().to_owned(),
                state: match directory.state() {
                    native::SourceGuidanceRecoveryDirectoryState::Missing => {
                        LocalSourceGuidanceRecoveryDirectoryState::Missing
                    }
                    native::SourceGuidanceRecoveryDirectoryState::IdentityMatched => {
                        LocalSourceGuidanceRecoveryDirectoryState::IdentityMatched
                    }
                    native::SourceGuidanceRecoveryDirectoryState::Replaced => {
                        LocalSourceGuidanceRecoveryDirectoryState::Replaced
                    }
                },
            })
            .collect();
        let report = LocalSourceGuidanceRecoveryPlanReport {
            repository_id,
            host,
            intent,
            digest: plan.digest().to_string(),
            files,
            directories,
        };
        Ok(Some(LocalSourceGuidanceRecoveryPlan {
            compilation,
            repository,
            native: plan,
            report,
        }))
    }

    /// Rebuild recovery from fresh selected sources and native observations, then execute it.
    ///
    /// Neither a retained plan handle nor JSON report can bypass this fresh capture
    /// or the application's exact reviewed-digest admission. An uncheckpointed
    /// matching after-image remains uncertain origin even after its reviewed restore.
    ///
    /// # Errors
    /// Rejects invalid/stale review or absent recovery before native effects. Native
    /// uncertainty retains its journal and requires read-back before another decision.
    pub fn execute_local_source_guidance_recovery(
        &self,
        request: LocalSourceGuidanceRecoveryExecutionRequest,
    ) -> Result<LocalSourceGuidanceRecoveryResult, ApiError> {
        let reviewed = reviewed_digest(self, &request.reviewed_digest)?;
        let fresh = self
            .plan_local_source_guidance_recovery(request.selection)?
            .ok_or_else(|| {
                admission_failure(
                    self,
                    RecoveryAbsent,
                    pale_application::ApplicationFailureDisposition::Blocked,
                    "source_guidance_recovery_absent",
                    "no pending recovery journal was observed in the selected root",
                )
            })?;
        let admitted = fresh
            .compilation
            .compilation
            .admit_source_guidance_recovery(
                &fresh.repository,
                fresh.report.host.application(),
                fresh.native.proposal(),
                &reviewed,
            )
            .map_err(|error| {
                let d = error.failure_disposition();
                admission_failure(
                    self,
                    error,
                    d,
                    "source_guidance_recovery_review_rejected",
                    "reviewed recovery differs from the current native state or selected scope",
                )
            })?;
        let result = fresh.native.execute(admitted).map_err(|error| {
            native_failure(
                self,
                error,
                NativeBoundary::Recovery,
                "source_guidance_recovery_failed",
            )
        })?;
        Ok(LocalSourceGuidanceRecoveryResult {
            repository_id: fresh.report.repository_id,
            host: fresh.report.host,
            intent: fresh.report.intent,
            recovery_digest: result.recovery_sha256.to_string(),
            changed_files: result.changed_files,
            removed_directories: result.removed_directories,
            retained_directories: result.retained_directories,
            coordination_effect: LocalSourceGuidanceCoordinationEffect::from_native(
                result.coordination_effect,
            ),
        })
    }
}
#[derive(Debug, thiserror::Error)]
#[error("no pending source-guidance recovery journal was observed")]
struct RecoveryAbsent;
