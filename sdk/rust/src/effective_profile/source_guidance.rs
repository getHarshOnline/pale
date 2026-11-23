//! Owned projection of application-selected Package guidance.

use pale_core::records::{DiagnosticEffectCertaintyV1, DiagnosticStageV1, LogicalRepositoryIdV1};

use crate::error::{application_diagnostic_policy, FailureSpecification};
use crate::{ApiError, EffectiveProfileCompilation};

/// Explicit host layout for Package guidance, without native-host admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceGuidanceHost {
    /// Codex skills and a proposed `AGENTS.md` contribution.
    Codex,
    /// Claude Code skills and a proposed shared `AGENTS.md` contribution.
    ClaudeCode,
}

impl SourceGuidanceHost {
    pub(crate) const fn application(self) -> pale_application::render::SourceGuidanceHost {
        match self {
            Self::Codex => pale_application::render::SourceGuidanceHost::Codex,
            Self::ClaudeCode => pale_application::render::SourceGuidanceHost::ClaudeCode,
        }
    }
}

/// Owned proposed file; its identity does not permit a filesystem write.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SourceGuidanceFile {
    rel_path: String,
    content: String,
    sha256: String,
}

impl SourceGuidanceFile {
    /// Repository-relative destination.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.rel_path
    }

    /// Exact proposed UTF-8 bytes.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// SHA-256 of the proposed bytes.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Source-compiled guidance and a separate proposed project-entry contribution.
///
/// This owns ordinary portable output bytes, not a runtime generation, native
/// provider binding, operation lease or saved materialization plan. Project
/// entrypoint ownership and fresh target baselines require a separate qualified
/// write path. No machine or native host has been observed by this projection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SourceGuidanceProjection {
    repository_id: String,
    host: SourceGuidanceHost,
    profile_digest: String,
    digest: String,
    entrypoint_path: String,
    entrypoint_contribution: String,
    files: Vec<SourceGuidanceFile>,
}

impl SourceGuidanceProjection {
    /// Logical selected repository, not a checkout path.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Explicitly selected native layout.
    #[must_use]
    pub fn host(&self) -> SourceGuidanceHost {
        self.host
    }

    /// Exact source-compiled profile identity.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Content identity of sources, selection and all proposed output.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Project instruction file to which the contribution belongs.
    #[must_use]
    pub fn entrypoint_path(&self) -> &str {
        &self.entrypoint_path
    }

    /// Proposed addition only; existing project instructions retain ownership.
    #[must_use]
    pub fn entrypoint_contribution(&self) -> &str {
        &self.entrypoint_contribution
    }

    /// Complete proposed files, excluding the separately owned entrypoint.
    #[must_use]
    pub fn files(&self) -> &[SourceGuidanceFile] {
        &self.files
    }
}

impl EffectiveProfileCompilation {
    /// Render the selected repository's private scope and ordered reading references.
    ///
    /// Configured machine paths appear only in this explicit output, never in
    /// the portable files returned by [`Self::source_guidance`]. Keep it within
    /// the project's disclosure scope. No source is read and no host is admitted.
    ///
    /// # Errors
    ///
    /// Rejects invalid or unselected repositories and incomplete read-only
    /// source bindings. The diagnostic omits private paths and source bodies.
    pub fn repository_context(&self, repository_id: &str) -> Result<String, ApiError> {
        let repository = LogicalRepositoryIdV1::parse(repository_id).map_err(|source| {
            projection_failure(
                &self.diagnostic_identity,
                source,
                pale_application::ApplicationFailureDisposition::InvalidInput,
                "repository identifier is invalid",
            )
        })?;
        self.compilation
            .repository_context(&repository)
            .map_err(|source| {
                let disposition = source.failure_disposition();
                projection_failure(
                    &self.diagnostic_identity,
                    source,
                    disposition,
                    "private repository context cannot be projected",
                )
            })
    }

    /// Render Package judgment for a host before a runtime deployment exists.
    ///
    /// Uses the retained admitted source owner without rereading source or
    /// observing a machine. The application owns selection, composition, host
    /// layout and refusal semantics. This method performs no write and cannot
    /// be substituted for runtime compilation or operation admission.
    ///
    /// # Errors
    ///
    /// Returns a structured diagnostic for an invalid or unselected repository,
    /// runtime-dependent selections, or an inconsistent content projection.
    pub fn source_guidance(
        &self,
        repository_id: &str,
        host: SourceGuidanceHost,
    ) -> Result<SourceGuidanceProjection, ApiError> {
        let repository = LogicalRepositoryIdV1::parse(repository_id).map_err(|source| {
            projection_failure(
                &self.diagnostic_identity,
                source,
                pale_application::ApplicationFailureDisposition::InvalidInput,
                "repository identifier is invalid",
            )
        })?;
        let projected = self
            .compilation
            .source_guidance(&repository, host.application())
            .map_err(|source| {
                let disposition = source.failure_disposition();
                let summary = match &source {
                    pale_application::render::SourceGuidanceError::RepositoryNotSelected => "repository is not selected by this effective profile",
                    pale_application::render::SourceGuidanceError::RuntimeSelection => "selected operations, recipes, modes or lifecycle behavior require runtime projection; select a source-guidance profile explicitly",
                    pale_application::render::SourceGuidanceError::Content(_) => "selected Package content cannot be projected faithfully",
                };
                projection_failure(&self.diagnostic_identity, source, disposition, summary)
            })?;
        Ok(SourceGuidanceProjection {
            repository_id: projected.repository_id().to_string(),
            host,
            profile_digest: projected.profile_digest().to_string(),
            digest: projected.digest().to_string(),
            entrypoint_path: projected.entrypoint_path().to_owned(),
            entrypoint_contribution: projected.entrypoint_contribution().to_owned(),
            files: projected
                .files()
                .iter()
                .map(|file| SourceGuidanceFile {
                    rel_path: file.rel_path().to_owned(),
                    content: file.content().to_owned(),
                    sha256: file.sha256().to_string(),
                })
                .collect(),
        })
    }
}

fn projection_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    source: impl std::error::Error + Send + Sync + 'static,
    disposition: pale_application::ApplicationFailureDisposition,
    summary: &'static str,
) -> ApiError {
    let (disposition, retry) = application_diagnostic_policy(disposition);
    ApiError::new_with_opaque_source(
        identity,
        source,
        FailureSpecification {
            code: "source_guidance_projection_rejected",
            stage: DiagnosticStageV1::Rendering,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec!["No files, runtime generations or invocation grants were created."],
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
