//! Compile captured configuration for rendering and later effect-free planning.
//!
//! [`PaleApi::compile_one_shot`] accepts workspace, Package, and private
//! configuration documents in one request. Use it when the caller needs
//! repository plan candidates, not just the profile inspection provided by
//! [`PaleApi::compile_effective_profile`]. Rendering happens in memory; this
//! call does not read the filesystem or apply the resulting plans.
//!
//! The result retains the admitted compilation behind an opaque handle and
//! exposes summaries and planning methods. Compiler IR, direct rendered bytes,
//! machine paths, ports, adapters, and resident lifecycle records remain private
//! to their owning crates.

use std::sync::Arc;

use pale_application::compile::{
    CapturedLifecycleGuidance, CapturedPrivateWorkspaceInputs, Compilation, CompileError,
};
use pale_application::render::RenderError;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticStageV1, DiagnosticSubjectV1Wire,
};

use crate::effective_profile::EffectiveProfileCompilationRequest;
use crate::error::{composition_failure_details, ApiError, FailureSpecification};
use crate::portable_workspace::{compile_portable_request, PortableWorkspaceRequest};
use crate::PaleApi;

mod planning;
mod summary;

pub use planning::{
    MachineLocalEffectCandidate, OneShotAdmittedPlan, OneShotBaselineEntry,
    OneShotBaselineObservation, OneShotBoundPlan, OneShotObservationTarget,
    OneShotObservationTargets, OneShotPlanCandidate, OneShotPlanKind, OneShotPlanningRequest,
    OneShotPriorMaterialization,
};
pub use summary::{ExecutionPathSummary, OneShotCompilationSummary, RepositorySummary};

/// One labelled Package or lifecycle-guidance document supplied to compilation.
///
/// The label is a portable source coordinate, never a host path. [`Self::new`]
/// intentionally performs no parsing: the complete request is admitted by
/// [`PaleApi::compile_one_shot`] so duplicate labels, bounds, and role-specific
/// syntax share one structured failure boundary. Singular private-role bodies
/// use the role-named arguments of [`OneShotPrivateInputs::new`] instead.
pub struct CompilationDocument {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for CompilationDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompilationDocument")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl CompilationDocument {
    /// Capture one logical label and its complete raw bytes.
    #[must_use]
    pub fn new(label: String, bytes: Vec<u8>) -> Self {
        Self { label, bytes }
    }

    pub(crate) fn into_parts(self) -> (String, Vec<u8>) {
        (self.label, self.bytes)
    }
}

/// Fixed-role private inputs for one portable compiler invocation.
///
/// Each singular field has exactly one language role. Lifecycle guidance is the
/// only repeated private family because the selected capabilities record names
/// a bounded set of guidance documents. Construction does not grant trust to a
/// guidance label or reveal a filesystem location.
pub struct OneShotPrivateInputs {
    capabilities: CompilationDocument,
    machine_binding: CompilationDocument,
    distribution_lock: CompilationDocument,
    lifecycle_guidance: Vec<CompilationDocument>,
}

impl std::fmt::Debug for OneShotPrivateInputs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotPrivateInputs")
            .field("capabilities", &self.capabilities)
            .field("machine_binding", &self.machine_binding)
            .field("distribution_lock", &self.distribution_lock)
            .field("lifecycle_guidance_count", &self.lifecycle_guidance.len())
            .finish()
    }
}

impl OneShotPrivateInputs {
    /// Bind raw documents to the compiler's three singular private roles.
    ///
    /// Each singular byte body is bound directly to the role named by its
    /// argument. Lifecycle labels remain meaningful because the capabilities
    /// document refers to that repeated family by name.
    #[must_use]
    pub fn new(
        capabilities: CompilationDocument,
        machine_binding: CompilationDocument,
        distribution_lock: CompilationDocument,
        lifecycle_guidance: Vec<CompilationDocument>,
    ) -> Self {
        Self {
            capabilities,
            machine_binding,
            distribution_lock,
            lifecycle_guidance,
        }
    }

    pub(crate) fn into_application(
        self,
    ) -> Result<
        CapturedPrivateWorkspaceInputs,
        pale_application::compile::CapturedCompilationInputError,
    > {
        let Self {
            capabilities,
            machine_binding,
            distribution_lock,
            lifecycle_guidance,
        } = self;
        let lifecycle_guidance = lifecycle_guidance
            .into_iter()
            .map(|document| CapturedLifecycleGuidance::new(document.label, document.bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let (capabilities_label, capabilities) = capabilities.into_parts();
        let (machine_binding_label, machine_binding) = machine_binding.into_parts();
        let (distribution_lock_label, distribution_lock) = distribution_lock.into_parts();
        CapturedPrivateWorkspaceInputs::new(
            pale_application::compile::CapturedPrivateDocument::new(
                capabilities_label,
                capabilities,
            )?,
            pale_application::compile::CapturedPrivateDocument::new(
                machine_binding_label,
                machine_binding,
            )?,
            pale_application::compile::CapturedPrivateDocument::new(
                distribution_lock_label,
                distribution_lock,
            )?,
            lifecycle_guidance,
        )
    }
}

/// Complete owned request for one path-free, portable one-shot compilation.
///
/// `project_id` and `execution_workspace_id` select declarations from the
/// canonical desired topology admitted by [`PortableWorkspaceRequest`].
/// `machine_id` selects the separately supplied machine binding. Package
/// documents are immutable source-tree projections; no member carries
/// filesystem authority or requests live machine admission.
pub struct OneShotCompilationRequest {
    effective_profile: EffectiveProfileCompilationRequest,
    workspace: PortableWorkspaceRequest,
}

impl std::fmt::Debug for OneShotCompilationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotCompilationRequest")
            .field("effective_profile", &self.effective_profile)
            .field("workspace", &self.workspace)
            .finish()
    }
}

impl OneShotCompilationRequest {
    /// Reuse the captured source families at the earlier effective-profile phase.
    pub(crate) fn into_profile_sources(
        self,
    ) -> (PortableWorkspaceRequest, EffectiveProfileCompilationRequest) {
        (self.workspace, self.effective_profile)
    }

    /// Assemble captured source families without parsing or normalizing them.
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private_inputs: OneShotPrivateInputs,
        package_documents: Vec<CompilationDocument>,
        workspace: PortableWorkspaceRequest,
    ) -> Self {
        Self {
            effective_profile: EffectiveProfileCompilationRequest::new(
                project_id,
                execution_workspace_id,
                machine_id,
                private_inputs,
                package_documents,
            ),
            workspace,
        }
    }
}

/// Opaque admitted result of one complete portable compiler invocation.
///
/// The handle retains the application-owned one-shot result behind shared
/// immutable ownership so inspection and planning consume the same admitted
/// identity. It is explicitly not a resident compiled generation: it has no
/// generation sequence, journal custody, publication, activation, operation,
/// provider observation, or native-effect authority.
#[derive(Clone)]
pub struct OneShotCompilation {
    pub(super) compilation: Arc<Compilation>,
    summary: OneShotCompilationSummary,
}

impl std::fmt::Debug for OneShotCompilation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotCompilation")
            .field("repository_count", &self.summary.repository_count())
            .field(
                "package_semantic_record_count",
                &self.summary.package_semantic_record_count(),
            )
            .field(
                "workspace_admission_evidence_count",
                &self.summary.workspace_admission_evidence_count(),
            )
            .finish_non_exhaustive()
    }
}

impl OneShotCompilation {
    /// Borrow the admitted application compilation for crate-private composition.
    #[cfg(feature = "native-local")]
    pub(crate) fn application(&self) -> &Compilation {
        &self.compilation
    }

    /// Return an owned flat summary of the admitted one-shot result.
    ///
    /// Repository order is the compiler's validated materialization order.
    /// The summary contains identities and counts only; it does not expose
    /// compiler IR, generated content, private configuration, or source bytes.
    #[must_use]
    pub fn summary(&self) -> OneShotCompilationSummary {
        self.summary.clone()
    }
}

impl PaleApi {
    /// Compile captured documents into a result that supports repository planning.
    ///
    /// Unlike [`Self::compile_effective_profile`], this method also renders
    /// repository outputs in memory and seals the complete one-shot compilation.
    /// It does not read the filesystem, observe a live machine, or apply a plan.
    /// Workspace admission, Package capture, private-record admission, semantic
    /// closure, rendering, and one-shot sealing reuse their application-owned
    /// implementations. All source bodies are consumed before this method
    /// returns. The compilation proves captured-source admission and
    /// deterministic rendering, and can be passed to [`Self::plan_one_shot`].
    /// Path readiness and activation eligibility remain false; no resident
    /// generation or native effect is created.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when a captured family exceeds its bound, a source
    /// record is malformed, a selected contract or release is incoherent, or an
    /// a Pale application invariant fails. Diagnostics never contain supplied
    /// source bytes; the exact typed application cause remains private to the
    /// error object.
    pub fn compile_one_shot(
        &self,
        request: OneShotCompilationRequest,
    ) -> Result<OneShotCompilation, ApiError> {
        let (workspace, effective_profile) = request.into_profile_sources();
        let workspace = compile_portable_request(&self.diagnostic_identity, workspace)?;
        let inputs = effective_profile
            .into_application()
            .map_err(|error| compilation_failure(&self.diagnostic_identity, error))?;
        let compilation =
            pale_application::compile::compile_captured_portable_inputs(inputs, workspace)
                .map_err(|error| compilation_failure(&self.diagnostic_identity, error))?;
        let summary = OneShotCompilationSummary::project(&compilation)
            .map_err(|error| summary_failure(&self.diagnostic_identity, error))?;
        Ok(OneShotCompilation {
            compilation: Arc::new(compilation),
            summary,
        })
    }
}

/// Map the application error algebra into one stable API diagnostic policy.
pub(crate) fn compilation_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: CompileError,
) -> ApiError {
    let (disposition, retry) =
        crate::error::application_diagnostic_policy(error.failure_disposition());
    let (code, stage, summary) = match &error {
        CompileError::MissingExecutionPaths { .. } => (
            "one_shot_execution_paths_missing",
            DiagnosticStageV1::SemanticAdmission,
            "Select at least one execution_path_ids entry in the execution workspace before rendering.",
        ),
        CompileError::LifecycleConflict { .. } => (
            "one_shot_lifecycle_conflict",
            DiagnosticStageV1::SemanticAdmission,
            "The selected execution path requires an event missing from session lifecycle_events.",
        ),
        CompileError::TerminalProvider { .. } => (
            "one_shot_terminal_provider_rejected",
            DiagnosticStageV1::SemanticAdmission,
            "The selected execution path cannot resolve a supported terminal provider.",
        ),
        CompileError::CapturedInput(_) => (
            "one_shot_source_rejected",
            DiagnosticStageV1::InputAdmission,
            "captured one-shot source was rejected",
        ),
        CompileError::SchemaRegistry(_) | CompileError::SchemaRegistryMismatch { .. } => (
            "one_shot_contract_rejected",
            DiagnosticStageV1::ContractValidation,
            "captured one-shot sources do not match the embedded contract registry",
        ),
        CompileError::PackageLoad(_) => (
            "one_shot_package_rejected",
            DiagnosticStageV1::PackageLoading,
            "captured Package source was rejected",
        ),
        CompileError::Workspace(_) | CompileError::Verify(_) => (
            "one_shot_private_input_rejected",
            DiagnosticStageV1::WorkspaceLoading,
            "captured private compiler input was rejected",
        ),
        CompileError::CompilerIdentityMismatch { .. }
        | CompileError::Semantic(_) => (
            "one_shot_semantics_rejected",
            DiagnosticStageV1::SemanticAdmission,
            "captured one-shot semantics are inconsistent",
        ),
        CompileError::CompositionInput(_)
            if disposition == DiagnosticDispositionV1::InvalidInput =>
        {
            (
                "one_shot_semantics_rejected",
                DiagnosticStageV1::SemanticAdmission,
                "Workspace composition declarations are inconsistent.",
            )
        }
        CompileError::Render(source)
            if matches!(&**source, RenderError::HostNameCollision { .. }) =>
        {
            (
                "one_shot_host_name_collision",
                DiagnosticStageV1::Rendering,
                "Selected Package resources generate the same host name.",
            )
        }
        CompileError::Render(_) => (
            "one_shot_render_failed",
            DiagnosticStageV1::Rendering,
            "one-shot rendering contradicted a Pale application invariant",
        ),
        CompileError::Plan(_)
        | CompileError::CompilationInvariant(_)
        | CompileError::CompositionInput(_) => (
            "one_shot_compilation_failed",
            DiagnosticStageV1::Compilation,
            "one-shot compilation contradicted a Pale application invariant",
        ),
        CompileError::NativeActionRegistry(_) => (
            "one_shot_action_registry_failed",
            DiagnosticStageV1::Internal,
            "the compiled native-action registry is incoherent",
        ),
    };
    let (subject, facts, help) = match &error {
        CompileError::MissingExecutionPaths { execution_workspace } => (
            Some(DiagnosticSubjectV1Wire {
                kind: "execution_workspace".into(),
                id: execution_workspace.to_string(),
            }),
            vec![DiagnosticFactV1Wire {
                key: "field".into(),
                value: "execution_path_ids".into(),
            }],
            vec!["Choose a declared execution path for this workspace, then compile again. Profile inspection remains available while the selection is unfinished."],
        ),
        CompileError::LifecycleConflict { execution_path, event, declared } => (
            Some(DiagnosticSubjectV1Wire {
                kind: "execution_path".into(),
                id: execution_path.to_string(),
            }),
            vec![
                DiagnosticFactV1Wire { key: "field".into(), value: "lifecycle_events".into() },
                DiagnosticFactV1Wire { key: "required_event".into(), value: event.as_str().into() },
                DiagnosticFactV1Wire { key: "declared_events".into(), value: if declared.is_empty() { "none".into() } else { declared.iter().map(|event| event.as_str()).collect::<Vec<_>>().join("|") } },
            ],
            vec!["Review the path's policy and session limits. Either select a path that fits those limits or explicitly permit the required event, then compile again."],
        ),
        CompileError::TerminalProvider { execution_path, .. } => (
            Some(DiagnosticSubjectV1Wire { kind: "execution_path".into(), id: execution_path.to_string() }),
            Vec::new(),
            vec!["Check the selected path's terminal provider binding and adapter, then compile again."],
        ),
        CompileError::CompositionInput(source)
            if disposition == DiagnosticDispositionV1::InvalidInput =>
        {
            let (subject, facts) = composition_failure_details(source);
            (
                subject,
                facts,
                vec!["Correct the rejected composition declaration and compile again."],
            )
        }
        CompileError::Render(source) => match &**source {
            RenderError::HostNameCollision {
                kind,
                name,
                first,
                second,
            } => (
                Some(DiagnosticSubjectV1Wire {
                    kind: "host_name".into(),
                    id: name.clone(),
                }),
                [
                    ("host_namespace", *kind),
                    ("first_resource", first.as_str()),
                    ("second_resource", second.as_str()),
                ]
                .into_iter()
                .map(|(key, value)| DiagnosticFactV1Wire {
                    key: key.into(),
                    value: value.into(),
                })
                .collect(),
                vec!["Select resources with distinct generated host names and compile again."],
            ),
            _ => (None, Vec::new(), Vec::new()),
        },
        _ => (None, Vec::new(), Vec::new()),
    };
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage,
            disposition,
            summary,
            subject,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help,
            facts,
        },
    )
}

fn summary_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: summary::SummaryProjectionError,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "one_shot_summary_failed",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "one-shot summary exceeded its owned projection bound",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
