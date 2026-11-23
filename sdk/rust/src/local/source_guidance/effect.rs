//! Fresh reviewed write orchestration and native effect-certainty projection.

use super::{LocalSourceGuidanceIntent, LocalSourceGuidancePlanRequest};
use crate::error::FailureSpecification;
use crate::{ApiError, PaleApi, SourceGuidanceHost};
use pale_adapter_fs::{AdvisoryLockEffect, FsError, SourceGuidancePublicationFailureState};
use pale_core::records::{
    ArtifactDigest, DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire,
    DiagnosticRetryV1, DiagnosticStageV1, DiagnosticSubjectV1Wire,
};

/// Persistent native coordination observed separately from guidance file changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSourceGuidanceCoordinationEffect {
    /// The existing coordination namespace was unchanged.
    Untouched,
    /// A persistent coordination effect was confirmed.
    Confirmed,
    /// A coordination effect could not be settled.
    OutcomeUnknown,
}
impl LocalSourceGuidanceCoordinationEffect {
    pub(super) fn from_native(value: AdvisoryLockEffect) -> Self {
        match value {
            AdvisoryLockEffect::Untouched => Self::Untouched,
            AdvisoryLockEffect::Confirmed => Self::Confirmed,
            AdvisoryLockEffect::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

/// Fresh source selection and the exact identity of its reviewed file proposal.
pub struct LocalSourceGuidanceWriteRequest {
    plan: LocalSourceGuidancePlanRequest,
    reviewed_digest: Option<String>,
}
impl std::fmt::Debug for LocalSourceGuidanceWriteRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSourceGuidanceWriteRequest")
            .field("plan", &self.plan)
            .field("explicit_review", &self.reviewed_digest.is_some())
            .finish()
    }
}
impl LocalSourceGuidanceWriteRequest {
    /// Select apply/remove and the reviewed proposal; construction performs no I/O.
    #[must_use]
    pub fn new(plan: LocalSourceGuidancePlanRequest, reviewed_digest: String) -> Self {
        Self {
            plan,
            reviewed_digest: Some(reviewed_digest),
        }
    }

    /// Apply the last proposal remembered for this repository and host.
    /// Current sources and target identities must still match that proposal.
    #[must_use]
    pub fn remembered(plan: LocalSourceGuidancePlanRequest) -> Self {
        Self {
            plan,
            reviewed_digest: None,
        }
    }
}

/// Confirmed native file publication; neither host use nor runtime activation.
#[derive(Debug, serde::Serialize)]
pub struct LocalSourceGuidanceWriteResult {
    repository_id: String,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceIntent,
    plan_digest: String,
    changed_files: usize,
    coordination_effect: LocalSourceGuidanceCoordinationEffect,
}
impl LocalSourceGuidanceWriteResult {
    /// Logical selected target.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    /// Explicit native host layout.
    #[must_use]
    pub const fn host(&self) -> SourceGuidanceHost {
        self.host
    }
    /// Applied reconciliation intent.
    #[must_use]
    pub const fn intent(&self) -> LocalSourceGuidanceIntent {
        self.intent
    }
    /// Reviewed plan actually rebuilt and admitted for this call.
    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }
    /// Confirmed changed files, excluding unchanged checks.
    #[must_use]
    pub const fn changed_files(&self) -> usize {
        self.changed_files
    }
    /// Persistent coordination effects are separate from target content changes.
    #[must_use]
    pub const fn coordination_effect(&self) -> LocalSourceGuidanceCoordinationEffect {
        self.coordination_effect
    }
}

impl PaleApi {
    /// Rebuild current source/target inputs and apply only the exact reviewed proposal.
    ///
    /// Reuses application admission and the confined native publisher. A pending
    /// recovery journal blocks normal writes. Successful return confirms complete
    /// file read-back, commit and journal cleanup, not native coding-host use.
    ///
    /// # Errors
    /// Rejects invalid/stale review before effects; native failures distinguish
    /// not-started, restored-before-images and uncertain state, with coordination
    /// effects reported independently. Uncertainty requires read-back before retry.
    pub fn apply_local_source_guidance(
        &self,
        request: LocalSourceGuidanceWriteRequest,
    ) -> Result<LocalSourceGuidanceWriteResult, ApiError> {
        let explicit = request
            .reviewed_digest
            .as_deref()
            .map(|value| reviewed_digest(self, value))
            .transpose()?;
        let fresh = self.plan_local_source_guidance(request.plan)?;
        let reviewed = match explicit {
            Some(value) => value,
            None => super::review::read(self, &fresh)?,
        };
        let changed = !fresh.report.publication_order.is_empty();
        let admitted = fresh.plan.admit(&reviewed).map_err(|error| {
            admission_failure(
                self,
                error,
                pale_application::ApplicationFailureDisposition::Blocked,
                "source_guidance_review_changed",
                "reviewed source-guidance plan differs from current sources or target state",
            )
        })?;
        let prepared =
            pale_adapter_fs::prepare_source_guidance_write(admitted).map_err(|error| {
                native_failure(
                    self,
                    error,
                    NativeBoundary::Preparation,
                    "source_guidance_write_rejected",
                )
            })?;
        let applied = prepared.publish_reported().map_err(|error| {
            let state = error.state();
            native_failure(
                self,
                error.into_source(),
                NativeBoundary::Publication(state),
                "source_guidance_publication_failed",
            )
        })?;
        let result = applied.commit().map_err(|error| {
            native_failure(
                self,
                error,
                NativeBoundary::Commit { changed },
                "source_guidance_commit_failed",
            )
        })?;
        Ok(LocalSourceGuidanceWriteResult {
            repository_id: fresh.report.repository_id,
            host: fresh.report.host,
            intent: fresh.report.intent,
            plan_digest: result.plan_sha256.to_string(),
            changed_files: result.changed_files,
            coordination_effect: LocalSourceGuidanceCoordinationEffect::from_native(
                result.coordination_effect,
            ),
        })
    }
}

pub(super) fn reviewed_digest(api: &PaleApi, input: &str) -> Result<ArtifactDigest, ApiError> {
    ArtifactDigest::parse(input).map_err(|error| {
        admission_failure(
            api,
            error,
            pale_application::ApplicationFailureDisposition::InvalidInput,
            "source_guidance_review_invalid",
            "reviewed guidance identity must be an exact SHA-256 digest",
        )
    })
}

pub(super) fn admission_failure<E: std::error::Error + Send + Sync + 'static>(
    api: &PaleApi,
    error: E,
    consequence: pale_application::ApplicationFailureDisposition,
    code: &'static str,
    summary: &'static str,
) -> ApiError {
    let (disposition, retry) = crate::error::application_diagnostic_policy(consequence);
    ApiError::new_with_opaque_source(
        &api.diagnostic_identity,
        error,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::Planning,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotStarted,
            request: None,
            notes: vec!["No guidance write or recovery effect was admitted."],
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

#[derive(Clone, Copy)]
pub(super) enum NativeBoundary {
    Inspection,
    Preparation,
    Publication(SourceGuidancePublicationFailureState),
    Commit { changed: bool },
    Recovery,
}

pub(super) fn native_failure(
    api: &PaleApi,
    error: FsError,
    boundary: NativeBoundary,
    code: &'static str,
) -> ApiError {
    let coordination = error.coordination_effect();
    let unknown = error.effect_visibility() == pale_adapter_fs::EffectVisibility::MayBeVisible
        || error.restoration_failed()
        || coordination == AdvisoryLockEffect::OutcomeUnknown
        || matches!(
            boundary,
            NativeBoundary::Commit { changed: true }
                | NativeBoundary::Publication(
                    SourceGuidancePublicationFailureState::OutcomeUnknown
                )
        );
    let restored = matches!(
        boundary,
        NativeBoundary::Publication(SourceGuidancePublicationFailureState::Restored)
    );
    let inspection = matches!(boundary, NativeBoundary::Inspection);
    let (disposition, retry, effect) = if unknown {
        (
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
        )
    } else {
        let blocked = matches!(
            root_cause(&error),
            FsError::Collision(..)
                | FsError::InvalidState(..)
                | FsError::Escape(..)
                | FsError::FileReadLimit { .. }
        );
        (
            if blocked {
                DiagnosticDispositionV1::Blocked
            } else {
                DiagnosticDispositionV1::OperationalFailure
            },
            if blocked {
                DiagnosticRetryV1::AfterStateChange
            } else {
                DiagnosticRetryV1::NotIndicated
            },
            if inspection {
                DiagnosticEffectCertaintyV1::NotApplicable
            } else if restored || coordination == AdvisoryLockEffect::Confirmed {
                DiagnosticEffectCertaintyV1::Confirmed
            } else {
                DiagnosticEffectCertaintyV1::NotStarted
            },
        )
    };
    let subject = match root_cause(&error) {
        FsError::Collision(path, _)
            if pale_core::paths::ConfinedRelativePath::parse(path).is_some() =>
        {
            Some(DiagnosticSubjectV1Wire {
                kind: "source_guidance_file".into(),
                id: path.clone(),
            })
        }
        _ => None,
    };
    let target = if unknown {
        "outcome_unknown"
    } else if restored {
        "restored_before_images"
    } else {
        "not_started"
    };
    let coord = match coordination {
        AdvisoryLockEffect::Untouched => "untouched",
        AdvisoryLockEffect::Confirmed => "confirmed",
        AdvisoryLockEffect::OutcomeUnknown => "outcome_unknown",
    };
    let notes = if unknown {
        vec!["Inspect the current files and any retained recovery journal before deciding whether to retry."]
    } else if restored {
        vec!["Files mutated by this call were restored to before-image bytes/modes; independent edits remain untouched. Inodes and persistent coordination may differ."]
    } else if inspection {
        vec!["Inspection wrote no files."]
    } else {
        vec!["No guidance target mutation was started; persistent coordination is reported separately."]
    };
    ApiError::new_with_opaque_source(
        &api.diagnostic_identity,
        error,
        FailureSpecification {
            code,
            stage: if inspection {
                DiagnosticStageV1::HostObservation
            } else {
                DiagnosticStageV1::Materialization
            },
            disposition,
            summary: if unknown {
                "guidance file or recovery outcome requires native reconciliation"
            } else if restored {
                "guidance publication failed and before-images were restored"
            } else {
                "guidance native state did not permit the requested operation"
            },
            subject,
            schema_coordinate: None,
            location: None,
            retry,
            effect,
            request: None,
            notes,
            help: Vec::new(),
            facts: vec![
                DiagnosticFactV1Wire {
                    key: "guidance_target_effect".into(),
                    value: target.into(),
                },
                DiagnosticFactV1Wire {
                    key: "coordination_effect".into(),
                    value: coord.into(),
                },
            ],
        },
    )
}
fn root_cause(error: &FsError) -> &FsError {
    match error {
        FsError::Coordination { source, .. }
        | FsError::EffectUncertain { source, .. }
        | FsError::CleanupFailed { source, .. }
        | FsError::RestoreFailed { source, .. } => root_cause(source),
        _ => error,
    }
}
