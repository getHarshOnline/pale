//! Read local inputs and perform explicitly requested local operations through the SDK.
//!
//! The `native-local` feature provides workspace capture and compilation,
//! confined action-input reads, control-store inspection, executable-binding
//! verification, and materialization. Host paths and private bindings stay
//! behind owned requests and opaque results. Reading an action input does not
//! execute it; inspecting or compiling a workspace does not apply its
//! configuration.
//!
//! The feature selects dependencies, not workspace trust, Pale Runtime assembly,
//! or permission to mutate files. Each effect requires its explicit SDK
//! operation and the authority checked by that operation.

use std::path::{Component, PathBuf};

mod action_input;
mod configuration_review;
mod control_storage;
mod doctor;
mod executable_identity;
mod materialization;
mod source_guidance;
mod workspace_source;
pub use action_input::LocalNativeActionInputRequest;
pub use configuration_review::LocalConfigurationImprovementRequest;
pub use control_storage::{
    LocalControlStorageInspection, LocalControlStorageInspectionRejection,
    LocalControlStorageInspectionRequest, LocalControlStorageInspectionState,
    LocalControlStorageProfileRequirement, LocalControlStoreKind,
};
pub use doctor::{DoctorInspection, LocalDoctorRequest};
pub use executable_identity::{CompiledInterfaceIdentity, DurablePublicationMechanism};
pub use materialization::{
    LocalMaterializationAction, LocalMaterializationAdmissionRequest,
    LocalMaterializationAdmittedRun, LocalMaterializationDrift,
    LocalMaterializationMachineEffectKind, LocalMaterializationOwnershipChange,
    LocalMaterializationOwnershipObservation, LocalMaterializationReceiptObservation,
    LocalMaterializationReconciliation, LocalMaterializationRunOutcome,
    LocalMaterializationRunResult, LocalMaterializationTargetAdmission,
    LocalMaterializationTargetOutcome, LocalMaterializationTargetResult, LocalOneShotPlan,
    LocalOneShotPlanCounts, LocalOneShotPlanPublication, LocalOneShotPlanRequest,
    LocalRepositoryStatus, LocalWorkspaceStatus,
};
pub use source_guidance::{
    LocalSourceGuidanceBaseline, LocalSourceGuidanceChange, LocalSourceGuidanceCoordinationEffect,
    LocalSourceGuidanceFile, LocalSourceGuidanceIntent, LocalSourceGuidancePlan,
    LocalSourceGuidancePlanReport, LocalSourceGuidancePlanRequest,
    LocalSourceGuidanceRecoveryDirectory, LocalSourceGuidanceRecoveryDirectoryState,
    LocalSourceGuidanceRecoveryEvidence, LocalSourceGuidanceRecoveryExecutionRequest,
    LocalSourceGuidanceRecoveryFile, LocalSourceGuidanceRecoveryIntent,
    LocalSourceGuidanceRecoveryPlan, LocalSourceGuidanceRecoveryPlanReport,
    LocalSourceGuidanceRecoveryRequest, LocalSourceGuidanceRecoveryResult,
    LocalSourceGuidanceWriteRequest, LocalSourceGuidanceWriteResult,
};
pub use workspace_source::WorkspaceSourceInspection;

use pale_application::ApplicationFailureDisposition;
use pale_core::records::{DiagnosticEffectCertaintyV1, DiagnosticStageV1};

use crate::error::{ApiError, FailureSpecification};
use crate::{
    CompilationDocument, EffectiveProfileCompilation, OneShotCompilation,
    OneShotCompilationRequest, OneShotCompilationSummary, OneShotPrivateInputs,
    PackageDistributionInspection, PackageDistributionRequest, PaleApi, PortableWorkspaceDocument,
    PortableWorkspaceRequest,
};

/// Owned UTF-8 locator for one local `pale-workspace.toml` file.
///
/// The value is an ephemeral host input, not a workspace identity or serialized
/// Pale contract. [`PaleApi::compile_local_one_shot`] requires an absolute,
/// parent-free spelling before converting it privately to a host path.
/// Debug output reveals only its byte length.
pub struct LocalWorkspaceLocator {
    manifest: String,
}

impl std::fmt::Debug for LocalWorkspaceLocator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalWorkspaceLocator")
            .field("manifest_byte_len", &self.manifest.len())
            .finish()
    }
}

impl LocalWorkspaceLocator {
    /// Capture one caller-selected local workspace-manifest spelling.
    ///
    /// Construction performs no filesystem access. Absolute-path and parent
    /// traversal checks occur at [`PaleApi::compile_local_one_shot`] so
    /// every rejection uses the façade's structured diagnostic boundary.
    #[must_use]
    pub fn new(manifest: String) -> Self {
        Self { manifest }
    }

    fn into_path(self) -> Result<PathBuf, pale_application::workspace::WorkspaceError> {
        let path = PathBuf::from(&self.manifest);
        if !path.is_absolute() {
            return Err(
                pale_application::workspace::WorkspaceError::NonAbsolutePath {
                    binding: "workspace manifest".to_owned(),
                    path: self.manifest,
                },
            );
        }
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(
                pale_application::workspace::WorkspaceError::EscapedReference(self.manifest),
            );
        }
        Ok(path)
    }
}

/// Complete source selection for a local effective-profile or one-shot compilation.
///
/// Logical selectors are resolved together from [`LocalWorkspaceLocator`]; a
/// caller cannot supply independently captured private, Package, or workspace
/// documents. Debug output retains selectors but redacts the host locator.
pub struct LocalOneShotCompilationRequest {
    workspace: LocalWorkspaceLocator,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

impl std::fmt::Debug for LocalOneShotCompilationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalOneShotCompilationRequest")
            .field("workspace", &self.workspace)
            .field("project_id", &self.project_id)
            .field("execution_workspace_id", &self.execution_workspace_id)
            .field("machine_id", &self.machine_id)
            .finish()
    }
}

impl LocalOneShotCompilationRequest {
    /// Bind one local manifest to its project, execution-workspace, and machine selectors.
    #[must_use]
    pub fn new(
        workspace: LocalWorkspaceLocator,
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
    ) -> Self {
        Self {
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        }
    }
}

/// Local source selection for Package distribution inspection.
///
/// This request uses the same manifest and logical selectors as local one-shot
/// compilation, but captures only the selected immutable Package tree. It does
/// not read a distribution lock or compile an effective workspace profile.
pub struct LocalPackageDistributionRequest {
    workspace: LocalWorkspaceLocator,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

impl std::fmt::Debug for LocalPackageDistributionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalPackageDistributionRequest")
            .field("workspace", &self.workspace)
            .field("project_id", &self.project_id)
            .field("execution_workspace_id", &self.execution_workspace_id)
            .field("machine_id", &self.machine_id)
            .finish()
    }
}

impl LocalPackageDistributionRequest {
    /// Bind one local manifest to the logical selection whose Package tree is inspected.
    #[must_use]
    pub fn new(
        workspace: LocalWorkspaceLocator,
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
    ) -> Self {
        Self {
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        }
    }
}

/// Retains a compiled workspace and its private host inputs for later local operations.
///
/// [`Self::compilation`] exposes the same [`OneShotCompilation`] used by callers
/// that supply captured documents. Host paths and bindings remain private and
/// tied to that capture. Later executable-binding and materialization checks
/// use those inputs without repeating workspace selection. Retaining them does
/// not prove process health or authorize a file change.
pub struct LocalOneShotCompilation {
    compilation: OneShotCompilation,
    host: pale_application::workspace::OneShotHostEffectContext,
}

impl std::fmt::Debug for LocalOneShotCompilation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalOneShotCompilation")
            .field("summary", &self.compilation.summary())
            .finish_non_exhaustive()
    }
}

impl LocalOneShotCompilation {
    /// Borrow the path-free admitted compilation shared with pure API use cases.
    #[must_use]
    pub const fn compilation(&self) -> &OneShotCompilation {
        &self.compilation
    }

    /// Return the bounded path-free summary of the admitted compilation.
    #[must_use]
    pub fn summary(&self) -> OneShotCompilationSummary {
        self.compilation.summary()
    }
}

impl PaleApi {
    /// Capture local sources and retain their effective profile without runtime rendering.
    ///
    /// This reuses the same bounded source-capture owner as one-shot compilation
    /// and configuration review. It observes no executable, host or repository
    /// readiness, produces no runtime generation and writes no files. Call
    /// [`EffectiveProfileCompilation::source_guidance`] to project selected
    /// judgment, or inspect the retained profile through the normal API.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the locator or selectors are invalid, bounded
    /// source capture fails or changes between reads, or workspace/profile
    /// admission rejects the captured bytes. Missing runtime paths do not
    /// prevent source admission; they still prevent runtime rendering.
    pub fn compile_local_effective_profile(
        &self,
        request: LocalOneShotCompilationRequest,
    ) -> Result<EffectiveProfileCompilation, ApiError> {
        let LocalOneShotCompilationRequest {
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        } = request;
        let sources = pale_adapter_fs::LocalSourceFilesystem;
        let captured = resolve_local_entry(
            &self.diagnostic_identity,
            &sources,
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        )?
        .capture_one_shot_sources(&sources)
        .map_err(|error| local_capture_failure(&self.diagnostic_identity, error))?;
        let (workspace, profile) = captured_request(captured).into_profile_sources();
        let workspace = self.compile_portable_workspace(workspace)?;
        self.compile_effective_profile(&workspace, profile)
    }

    /// Resolve, stably capture, and compile one local workspace selection.
    ///
    /// Resolution and bounded double-read capture reuse the application-owned
    /// canonical entry. The resulting source bodies are consumed by the same
    /// [`Self::compile_one_shot`] path used by binding leaves; no parser,
    /// validator, compiler, or summary policy is duplicated here. The retained
    /// host context is opaque. Compilation does not verify executable bindings
    /// or write files.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the locator is not absolute and parent-free,
    /// local source observation fails, the selected declarations disagree, the
    /// bounded capture changes between reads, or ordinary one-shot admission
    /// and compilation rejects the captured source.
    pub fn compile_local_one_shot(
        &self,
        request: LocalOneShotCompilationRequest,
    ) -> Result<LocalOneShotCompilation, ApiError> {
        let LocalOneShotCompilationRequest {
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        } = request;
        let sources = pale_adapter_fs::LocalSourceFilesystem;
        let entry = resolve_local_entry(
            &self.diagnostic_identity,
            &sources,
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        )?;
        let (captured, host) = entry
            .capture_one_shot_with_host_context(&sources)
            .map_err(|error| local_capture_failure(&self.diagnostic_identity, error))?;
        let compilation = self.compile_one_shot(captured_request(captured))?;
        Ok(LocalOneShotCompilation { compilation, host })
    }

    /// Inspect the Package releases selected by one local workspace entry.
    ///
    /// Resolution and bounded Package capture reuse the application-owned local
    /// entry. The captured path-free documents then enter
    /// [`Self::inspect_package_distribution`], keeping Package admission and
    /// release-coordinate derivation single-owned. No lock, semantic profile,
    /// generated surface, or runtime state is read or constructed.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when local selection or bounded Package capture
    /// fails, or the captured distribution is not admissible.
    pub fn inspect_local_package_distribution(
        &self,
        request: LocalPackageDistributionRequest,
    ) -> Result<PackageDistributionInspection, ApiError> {
        let LocalPackageDistributionRequest {
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        } = request;
        let sources = pale_adapter_fs::LocalSourceFilesystem;
        let entry = resolve_local_entry(
            &self.diagnostic_identity,
            &sources,
            workspace,
            project_id,
            execution_workspace_id,
            machine_id,
        )?;
        let documents = entry
            .capture_package_sources(&sources)
            .map_err(|error| local_package_failure(&self.diagnostic_identity, error))?
            .into_iter()
            .map(compilation_document)
            .collect();
        self.inspect_package_distribution(PackageDistributionRequest::new(documents))
    }
}

fn resolve_local_entry(
    identity: &pale_core::records::RecordSchemaIdentity,
    sources: &pale_adapter_fs::LocalSourceFilesystem,
    workspace: LocalWorkspaceLocator,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
) -> Result<pale_application::workspace::ResolvedEntry, ApiError> {
    let manifest = workspace
        .into_path()
        .map_err(|error| local_workspace_failure(identity, error))?;
    pale_application::workspace::resolve_entry(
        sources,
        &manifest,
        &project_id,
        &execution_workspace_id,
        &machine_id,
    )
    .map_err(|error| local_workspace_failure(identity, error))
}

fn captured_request(
    captured: pale_application::workspace::CapturedOneShotSources,
) -> OneShotCompilationRequest {
    let parts = captured.into_parts();
    OneShotCompilationRequest::new(
        parts.project_id,
        parts.execution_workspace_id,
        parts.machine_id,
        captured_private_inputs(
            parts.capabilities,
            parts.machine_binding,
            parts.distribution_lock,
            parts.lifecycle_guidance,
        ),
        parts
            .package_documents
            .into_iter()
            .map(compilation_document)
            .collect(),
        captured_workspace(parts.workspace_root_manifest, parts.workspace_documents),
    )
}

/// Translate captured private roles without reopening their source labels.
fn captured_private_inputs(
    capabilities: pale_application::workspace::CapturedOneShotSourceDocument,
    machine_binding: pale_application::workspace::CapturedOneShotSourceDocument,
    distribution_lock: pale_application::workspace::CapturedOneShotSourceDocument,
    lifecycle_guidance: Vec<pale_application::workspace::CapturedOneShotSourceDocument>,
) -> OneShotPrivateInputs {
    OneShotPrivateInputs::new(
        compilation_document(capabilities),
        compilation_document(machine_binding),
        compilation_document(distribution_lock),
        lifecycle_guidance
            .into_iter()
            .map(compilation_document)
            .collect(),
    )
}

/// Preserve the root label and document order of a captured workspace.
fn captured_workspace(
    root: String,
    documents: Vec<pale_application::workspace::CapturedOneShotSourceDocument>,
) -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new(
        root,
        documents.into_iter().map(workspace_document).collect(),
    )
}

fn compilation_document(
    document: pale_application::workspace::CapturedOneShotSourceDocument,
) -> CompilationDocument {
    let (label, bytes) = document.into_parts();
    CompilationDocument::new(label, bytes)
}

fn workspace_document(
    document: pale_application::workspace::CapturedOneShotSourceDocument,
) -> PortableWorkspaceDocument {
    let (label, bytes) = document.into_parts();
    PortableWorkspaceDocument::new(label, bytes)
}

fn local_workspace_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: pale_application::workspace::WorkspaceError,
) -> ApiError {
    let disposition = error.failure_disposition();
    local_host_failure(
        identity,
        error,
        disposition,
        DiagnosticStageV1::WorkspaceLoading,
        "local_workspace_rejected",
        "local workspace resolution was rejected",
    )
}

fn local_capture_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: pale_application::workspace::OneShotSourceCaptureError,
) -> ApiError {
    let disposition = match &error {
        pale_application::workspace::OneShotSourceCaptureError::Workspace(source) => {
            source.failure_disposition()
        }
        pale_application::workspace::OneShotSourceCaptureError::Package(source) => {
            source.failure_disposition()
        }
    };
    local_host_failure(
        identity,
        error,
        disposition,
        DiagnosticStageV1::HostObservation,
        "local_workspace_capture_failed",
        "local workspace sources could not be captured stably",
    )
}

fn local_package_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: pale_application::packages::PackageError,
) -> ApiError {
    let disposition = error.failure_disposition();
    local_host_failure(
        identity,
        error,
        disposition,
        DiagnosticStageV1::HostObservation,
        "local_package_capture_failed",
        "the selected Package distribution could not be captured stably",
    )
}

fn local_host_failure<E>(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: E,
    disposition: ApplicationFailureDisposition,
    stage: DiagnosticStageV1,
    code: &'static str,
    summary: &'static str,
) -> ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    let (disposition, retry) = crate::error::application_diagnostic_policy(disposition);
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
