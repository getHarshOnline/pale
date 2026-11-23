//! Local source capture for a review, without rendering or writing configuration.

use crate::{
    ApiError, ConfigurationImprovementPreview, ConfigurationImprovementProposalRequest,
    ConfigurationImprovementSources, EffectiveProfileCompilationRequest, LocalWorkspaceLocator,
    PaleApi,
};

use super::{
    captured_private_inputs, captured_workspace, compilation_document, local_capture_failure,
    local_workspace_failure, resolve_local_entry,
};

/// Two local manifests reviewed under the same project, execution workspace and machine.
///
/// Both manifests must be absolute, parent-free locators. They are inputs, not
/// workspace identities. Candidate Package files are not read: the review uses
/// the baseline's Package source. Debug output omits paths and selector values.
pub struct LocalConfigurationImprovementRequest {
    baseline_workspace: LocalWorkspaceLocator,
    candidate_workspace: LocalWorkspaceLocator,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

impl std::fmt::Debug for LocalConfigurationImprovementRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalConfigurationImprovementRequest")
            .field("baseline_workspace", &self.baseline_workspace)
            .field("candidate_workspace", &self.candidate_workspace)
            .field("project_id_byte_len", &self.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.machine_id.len())
            .finish()
    }
}

impl LocalConfigurationImprovementRequest {
    /// Select two source roots and one common review context without reading them.
    #[must_use]
    pub fn new(
        baseline_workspace: LocalWorkspaceLocator,
        candidate_workspace: LocalWorkspaceLocator,
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
    ) -> Self {
        Self {
            baseline_workspace,
            candidate_workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        }
    }
}

impl PaleApi {
    /// Capture two local configurations and return their compiled review preview.
    ///
    /// This is the local-file entry to the same review used by captured-source
    /// callers. The candidate reuses the baseline's Package meaning. The
    /// returned preview supports the existing profile, impact and explanation
    /// queries and consumes one explicit review decision; it never writes
    /// configuration. Source changes after capture do not update the preview.
    ///
    /// # Errors
    ///
    /// Returns the unchanged [`ApiError`] from
    /// [`Self::capture_local_configuration_improvement_sources`] or
    /// [`Self::preview_configuration_improvement_sources`]. Reading both roots
    /// does not establish an atomic filesystem or live-editor snapshot.
    pub fn preview_local_configuration_improvement(
        &self,
        request: LocalConfigurationImprovementRequest,
    ) -> Result<ConfigurationImprovementPreview, ApiError> {
        self.capture_local_configuration_improvement_sources(request)
            .and_then(|sources| self.preview_configuration_improvement_sources(sources))
    }

    /// Capture local baseline and candidate inputs for a fresh, effect-free review.
    ///
    /// Each source set is bounded and rechecked by the application capture
    /// owner. Capture does not promise an atomic filesystem snapshot across
    /// both roots or establish current editor-buffer contents. Semantic review
    /// occurs in [`Self::preview_configuration_improvement_sources`].
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when either locator or common selection is invalid,
    /// a required source is absent, a read exceeds its bound, or repeated reads
    /// disagree. It performs no rendering, publication or configuration write.
    pub fn capture_local_configuration_improvement_sources(
        &self,
        request: LocalConfigurationImprovementRequest,
    ) -> Result<ConfigurationImprovementSources, ApiError> {
        let LocalConfigurationImprovementRequest {
            baseline_workspace,
            candidate_workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        } = request;
        let sources = pale_adapter_fs::LocalSourceFilesystem;
        let baseline = resolve_local_entry(
            &self.diagnostic_identity,
            &sources,
            baseline_workspace,
            project_id.clone(),
            execution_workspace_id.clone(),
            machine_id.clone(),
        )?
        .capture_one_shot_sources(&sources)
        .map_err(|error| local_capture_failure(&self.diagnostic_identity, error))?
        .into_parts();
        let candidate = resolve_local_entry(
            &self.diagnostic_identity,
            &sources,
            candidate_workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        )?
        .capture_configuration_improvement_candidate_sources(&sources)
        .map_err(|error| local_workspace_failure(&self.diagnostic_identity, error))?
        .into_parts();
        Ok(ConfigurationImprovementSources::new(
            captured_workspace(
                baseline.workspace_root_manifest,
                baseline.workspace_documents,
            ),
            EffectiveProfileCompilationRequest::new(
                baseline.project_id,
                baseline.execution_workspace_id,
                baseline.machine_id,
                captured_private_inputs(
                    baseline.capabilities,
                    baseline.machine_binding,
                    baseline.distribution_lock,
                    baseline.lifecycle_guidance,
                ),
                baseline
                    .package_documents
                    .into_iter()
                    .map(compilation_document)
                    .collect(),
            ),
            ConfigurationImprovementProposalRequest::new(
                candidate.project_id,
                candidate.execution_workspace_id,
                candidate.machine_id,
                captured_workspace(
                    candidate.workspace_root_manifest,
                    candidate.workspace_documents,
                ),
                captured_private_inputs(
                    candidate.capabilities,
                    candidate.machine_binding,
                    candidate.distribution_lock,
                    candidate.lifecycle_guidance,
                ),
            ),
        ))
    }
}
