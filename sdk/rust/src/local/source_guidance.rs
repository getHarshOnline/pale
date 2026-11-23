//! Native capture for application-owned source-guidance plans.
//!
//! Two bounded observations reject changes witnessed during capture. They are
//! not a filesystem snapshot or an effect admission. The opaque result keeps
//! native identities private; its explicit report contains proposed full file
//! bodies, including preserved project text, for local review.

use std::collections::BTreeMap;
use std::path::Path;

use pale_adapter_fs::{observe_machine_local_file, ConfinedRoot, FsError, ReadLimit};
use pale_application::{
    plan::source_guidance::{
        self, GuidanceFileChange, GuidancePlanError, GuidancePlanIntent, SourceGuidancePlan,
        MAX_GUIDANCE_FILE_BYTES, MAX_GUIDANCE_PLAN_BYTES, MAX_OWNERSHIP_BYTES,
    },
    ApplicationFailureDisposition,
};
use pale_core::records::{
    ArtifactDigest, DiagnosticEffectCertaintyV1, DiagnosticStageV1, DiagnosticSubjectV1Wire,
    LogicalRepositoryIdV1,
};

use super::{local_host_failure, LocalOneShotCompilationRequest};
use crate::error::{application_diagnostic_policy, FailureSpecification};
use crate::{ApiError, EffectiveProfileCompilation, PaleApi, SourceGuidanceHost};

mod effect;
mod recovery;
mod review;
pub use effect::{
    LocalSourceGuidanceCoordinationEffect, LocalSourceGuidanceWriteRequest,
    LocalSourceGuidanceWriteResult,
};
pub use recovery::{
    LocalSourceGuidanceRecoveryDirectory, LocalSourceGuidanceRecoveryDirectoryState,
    LocalSourceGuidanceRecoveryEvidence, LocalSourceGuidanceRecoveryExecutionRequest,
    LocalSourceGuidanceRecoveryFile, LocalSourceGuidanceRecoveryIntent,
    LocalSourceGuidanceRecoveryPlan, LocalSourceGuidanceRecoveryPlanReport,
    LocalSourceGuidanceRecoveryRequest, LocalSourceGuidanceRecoveryResult,
};

/// Requested source reconciliation, independent of runtime plan classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSourceGuidanceIntent {
    /// Reconcile selected methods and remove superseded owned files.
    Apply,
    /// Remove this host's owned guidance and exact entry contribution.
    Remove,
}

impl LocalSourceGuidanceIntent {
    const fn application(self) -> GuidancePlanIntent {
        match self {
            Self::Apply => GuidancePlanIntent::Apply,
            Self::Remove => GuidancePlanIntent::Remove,
        }
    }
}

/// Explicit native source selection and reconciliation target.
#[derive(Debug)]
pub struct LocalSourceGuidancePlanRequest {
    selection: LocalOneShotCompilationRequest,
    repository_id: String,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceIntent,
}

impl LocalSourceGuidancePlanRequest {
    /// Bind source selectors to one logical repository and explicit host layout.
    #[must_use]
    pub fn new(
        selection: LocalOneShotCompilationRequest,
        repository_id: String,
        host: SourceGuidanceHost,
        intent: LocalSourceGuidanceIntent,
    ) -> Self {
        Self {
            selection,
            repository_id,
            host,
            intent,
        }
    }
}

/// File presence, byte identity and mode observed before reconciliation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocalSourceGuidanceBaseline {
    /// Neither an existing file nor its bytes were observed.
    Absent,
    /// An observed regular file; its native inode stays in the opaque plan.
    Present {
        /// SHA-256 of exact observed bytes.
        sha256: String,
        /// Native Unix permissions.
        mode: u32,
    },
}

/// Proposed file change, without write authority.
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocalSourceGuidanceChange {
    /// Preserve this path's current bytes, mode or absence.
    Keep,
    /// Publish a complete file; project-entry content can contain local facts.
    Write {
        /// Exact proposed UTF-8 bytes, including preserved native text.
        content: String,
        /// SHA-256 of the proposed complete bytes.
        sha256: String,
        /// Proposed Unix permissions.
        mode: u32,
    },
    /// Delete only this previously owned file.
    Delete,
}

/// One path's observed baseline and proposed change.
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct LocalSourceGuidanceFile {
    rel_path: String,
    baseline: LocalSourceGuidanceBaseline,
    previous_content: Option<String>,
    change: LocalSourceGuidanceChange,
}

impl LocalSourceGuidanceFile {
    /// Observed UTF-8 text for a readable diff; absent for missing or non-text files.
    #[must_use]
    pub fn previous_content(&self) -> Option<&str> {
        self.previous_content.as_deref()
    }
    /// Confined path beneath the selected repository.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.rel_path
    }

    /// Observed byte identity and mode, or absence.
    #[must_use]
    pub const fn baseline(&self) -> &LocalSourceGuidanceBaseline {
        &self.baseline
    }

    /// Complete proposed effect; no native call has executed it.
    #[must_use]
    pub const fn change(&self) -> &LocalSourceGuidanceChange {
        &self.change
    }
}

#[cfg(all(test, unix))]
#[path = "../../tests/internal/local_source_guidance.rs"]
mod tests;

/// Explicit local review output, never accepted as a saved runtime plan.
///
/// Serialization is deliberate: proposed entrypoint bytes include preserved
/// local project text. Keep this report within that project's disclosure scope.
/// It contains no absolute root path, inode, invocation grant or provider lease.
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct LocalSourceGuidancePlanReport {
    repository_id: String,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceIntent,
    digest: String,
    files: Vec<LocalSourceGuidanceFile>,
    publication_order: Vec<String>,
}

impl LocalSourceGuidancePlanReport {
    /// Logical repository selected for reconciliation.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Explicit host layout, not an observation of installed readiness.
    #[must_use]
    pub const fn host(&self) -> SourceGuidanceHost {
        self.host
    }

    /// Requested apply or removal proposal.
    #[must_use]
    pub const fn intent(&self) -> LocalSourceGuidanceIntent {
        self.intent
    }

    /// Exact identity binding source selection, native root and observed baselines.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// All observed paths, including entrypoint and private ownership state.
    #[must_use]
    pub fn files(&self) -> &[LocalSourceGuidanceFile] {
        &self.files
    }

    /// Changed targets in exact application-owned order, covered by the digest.
    #[must_use]
    pub fn publication_order(&self) -> &[String] {
        &self.publication_order
    }
}

/// Opaque native proposal retaining exact baselines, without effect admission.
pub struct LocalSourceGuidancePlan {
    plan: SourceGuidancePlan,
    report: LocalSourceGuidancePlanReport,
}

impl std::fmt::Debug for LocalSourceGuidancePlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalSourceGuidancePlan")
            .field("repository_id", &self.report.repository_id)
            .field("host", &self.report.host)
            .field("intent", &self.report.intent)
            .field("digest", &self.plan.digest())
            .field("files", &self.report.files.len())
            .finish_non_exhaustive()
    }
}

impl LocalSourceGuidancePlan {
    /// Exact owned report for deliberate local review or serialization.
    #[must_use]
    pub const fn report(&self) -> &LocalSourceGuidancePlanReport {
        &self.report
    }
}

impl PaleApi {
    /// Capture sources and target baselines, then plan source-guidance reconciliation.
    ///
    /// Reads only the selected canonical checkout through the confined native
    /// adapter. State is bounded to 1 MiB, individual files to 8 MiB, and observed
    /// plus proposed payloads to the application's 64 MiB budget, excluding
    /// report copies and other process memory. Every
    /// captured path is observed again before completing the proposal. No lock,
    /// directory, ownership record, target file or runtime generation is written.
    ///
    /// # Errors
    ///
    /// Rejects invalid source selections, unselected/read-only repositories,
    /// runtime-dependent apply selections, unsafe or oversized targets, changed
    /// observations, malformed ownership and unowned/edited destination collisions.
    pub fn plan_local_source_guidance(
        &self,
        request: LocalSourceGuidancePlanRequest,
    ) -> Result<LocalSourceGuidancePlan, ApiError> {
        let LocalSourceGuidancePlanRequest {
            selection,
            repository_id,
            host,
            intent,
        } = request;
        let (compilation, repository, root) = source_context(self, selection, &repository_id)?;
        let state_path = source_guidance::ownership_path(host.application());
        let state =
            observe_machine_local_file(&root, &state_path, ReadLimit::new(MAX_OWNERSHIP_BYTES))
                .map_err(|error| observation_failure(self, error))?;
        let preparation = source_guidance::prepare_source_guidance(
            &compilation.compilation,
            &repository,
            host.application(),
            intent.application(),
            state.clone(),
        )
        .map_err(|error| planning_failure(self, error))?;
        let mut observed_bytes = state.bytes.as_ref().map_or(0, Vec::len);
        let mut observations = BTreeMap::new();
        for path in preparation.required_paths() {
            let remaining = MAX_GUIDANCE_PLAN_BYTES.saturating_sub(observed_bytes);
            let observed = observe_machine_local_file(
                &root,
                &path,
                ReadLimit::new(MAX_GUIDANCE_FILE_BYTES.min(remaining)),
            )
            .map_err(|error| observation_failure(self, error))?;
            observed_bytes += observed.bytes.as_ref().map_or(0, Vec::len);
            observations.insert(path, observed);
        }
        for (path, baseline) in std::iter::once((&state_path, &state)).chain(observations.iter()) {
            let limit = if path == &state_path {
                MAX_OWNERSHIP_BYTES
            } else {
                MAX_GUIDANCE_FILE_BYTES
            };
            let fresh = observe_machine_local_file(&root, path, ReadLimit::new(limit))
                .map_err(|error| observation_failure(self, error))?;
            if baseline != &fresh {
                return Err(planning_failure(
                    self,
                    GuidancePlanError::Baseline("target changed during capture"),
                ));
            }
        }
        root.verify_path_identity()
            .map_err(|error| observation_failure(self, error))?;
        let plan = preparation
            .finish(observations)
            .map_err(|error| planning_failure(self, error))?;
        let files = plan
            .files()
            .iter()
            .map(|file| {
                LocalSourceGuidanceFile::from_native(
                    file.rel_path(),
                    file.baseline(),
                    file.change(),
                )
            })
            .collect();
        let report = LocalSourceGuidancePlanReport {
            repository_id,
            host,
            intent,
            digest: plan.digest().to_string(),
            files,
            publication_order: plan.publication_order().to_vec(),
        };
        Ok(LocalSourceGuidancePlan { plan, report })
    }
}

fn planning_failure(api: &PaleApi, error: GuidancePlanError) -> ApiError {
    let (disposition, retry) = application_diagnostic_policy(error.failure_disposition());
    let mut subject = None;
    let mut notes = vec!["No files, locks or ownership state were written."];
    let summary = match &error {
        GuidancePlanError::Projection(pale_application::render::SourceGuidanceError::RepositoryNotSelected) =>
            "repository is not selected by this effective profile",
        GuidancePlanError::Projection(pale_application::render::SourceGuidanceError::RuntimeSelection) =>
            "selected runtime behavior requires the admitted runtime path; select a source-guidance profile explicitly",
        GuidancePlanError::Projection(pale_application::render::SourceGuidanceError::Content(_)) =>
            "selected Package guidance cannot be projected faithfully",
        GuidancePlanError::ReadOnly => "selected repository permits observation only",
        GuidancePlanError::Baseline(reason) => {
            notes.push(reason);
            "source-guidance target observations are inconsistent"
        }
        GuidancePlanError::Ownership(reason) => {
            notes.push(reason);
            "source-guidance ownership state cannot be reconciled"
        }
        GuidancePlanError::Collision { path, reason } => {
            subject = Some(DiagnosticSubjectV1Wire { kind: "source_guidance_file".into(), id: path.clone() });
            notes.push(reason);
            "source-guidance would overwrite unowned or edited content"
        }
        GuidancePlanError::Limit(bound) => {
            notes.push(bound);
            "source-guidance exceeds its bounded planning budget"
        }
        GuidancePlanError::Decode(_) => "source-guidance ownership state is malformed",
        GuidancePlanError::Encode(_) | GuidancePlanError::Canonical(_) =>
            "source-guidance proposal could not be encoded consistently",
    };
    // Only application-authored reasons and confined paths cross this boundary.
    // Decoder and filesystem error strings remain opaque.
    let specification = FailureSpecification {
        code: "source_guidance_plan_rejected",
        stage: DiagnosticStageV1::Planning,
        disposition,
        summary,
        subject,
        schema_coordinate: None,
        location: None,
        retry,
        effect: DiagnosticEffectCertaintyV1::NotApplicable,
        request: None,
        notes,
        help: Vec::new(),
        facts: Vec::new(),
    };
    ApiError::new_with_opaque_source(&api.diagnostic_identity, error, specification)
}

fn observation_failure(api: &PaleApi, error: FsError) -> ApiError {
    local_host_failure(
        &api.diagnostic_identity,
        error,
        ApplicationFailureDisposition::Operational,
        DiagnosticStageV1::HostObservation,
        "source_guidance_observation_failed",
        "source-guidance target could not be captured stably through the native filesystem",
    )
}

fn source_context(
    api: &PaleApi,
    selection: LocalOneShotCompilationRequest,
    repository_id: &str,
) -> Result<
    (
        EffectiveProfileCompilation,
        LogicalRepositoryIdV1,
        ConfinedRoot,
    ),
    ApiError,
> {
    let repository = LogicalRepositoryIdV1::parse(repository_id).map_err(|error| {
        local_host_failure(
            &api.diagnostic_identity,
            error,
            ApplicationFailureDisposition::InvalidInput,
            DiagnosticStageV1::InputAdmission,
            "source_guidance_repository_rejected",
            "source-guidance repository identifier is invalid",
        )
    })?;
    let compilation = api.compile_local_effective_profile(selection)?;
    if !compilation
        .compilation
        .profile()
        .repositories
        .iter()
        .any(|r| r.id == repository)
    {
        return Err(planning_failure(
            api,
            GuidancePlanError::Projection(
                pale_application::render::SourceGuidanceError::RepositoryNotSelected,
            ),
        ));
    }
    let root_path = compilation
        .compilation
        .instance()
        .repositories
        .get(&repository)
        .ok_or_else(|| {
            planning_failure(
                api,
                GuidancePlanError::Baseline("selected checkout is absent"),
            )
        })?;
    let root = ConfinedRoot::open(Path::new(root_path))
        .map_err(|error| observation_failure(api, error))?;
    if root.canonical_path().to_str() != Some(root_path.as_str()) {
        return Err(planning_failure(
            api,
            GuidancePlanError::Baseline("selected checkout must be canonical"),
        ));
    }
    Ok((compilation, repository, root))
}

impl LocalSourceGuidanceFile {
    fn from_native(
        path: &str,
        baseline: &pale_ports::MachineLocalFileObservation,
        change: &GuidanceFileChange,
    ) -> Self {
        Self {
            rel_path: path.to_owned(),
            previous_content: baseline
                .bytes
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned),
            baseline: match &baseline.prior {
                None => LocalSourceGuidanceBaseline::Absent,
                Some(prior) => LocalSourceGuidanceBaseline::Present {
                    sha256: prior.sha256.to_string(),
                    mode: prior.mode.get(),
                },
            },
            change: match change {
                GuidanceFileChange::Keep => LocalSourceGuidanceChange::Keep,
                GuidanceFileChange::Delete => LocalSourceGuidanceChange::Delete,
                GuidanceFileChange::Write { content, mode } => LocalSourceGuidanceChange::Write {
                    content: content.clone(),
                    sha256: ArtifactDigest::hash_bytes(content.as_bytes()).to_string(),
                    mode: mode.get(),
                },
            },
        }
    }
}
