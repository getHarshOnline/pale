//! Typed MCP inputs translated into the shared command vocabulary.

use pale_cli::command::{
    CompilationDocument, ConfigurationImprovementCommand, ConfigurationImprovementDisposition,
    ConfigurationImprovementPreviewSummary, ConfigurationImprovementProposalRequest,
    ConfigurationImprovementSources, ContractDocument, EffectiveProfileCompilationRequest,
    EffectiveProfileExplanationHandle,
    EffectiveProfileExplanationRequest as DispatcherProfileExplanationRequest,
    EffectiveProfileInspectionRequest as DispatcherProfileInspectionRequest,
    EffectiveProfilePageRequest, EffectiveProfileSection, OneShotBaselineEntry,
    OneShotBaselineObservation, OneShotCompilationRequest, OneShotPlanKind, OneShotPlanningRequest,
    OneShotPriorMaterialization, OneShotPrivateInputs, OperationHandleLocator,
    PortableWorkspaceDocument, PortableWorkspaceRequest, PortableWorkspaceSubjectSelector,
    ResidentConsultAdvanceRequest, ResidentConsultStartRequest, ResidentDelegateAdvanceRequest,
    ResidentDelegateStartRequest, ResidentInvokeAdvanceRequest, ResidentInvokeStartRequest,
    ResidentOperationCancellationRequest, ResidentOperationObservationRequest,
};
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

use pale_cli::command::{
    CommandRequest, PortableWorkspaceExplanationRequest as DispatcherExplanationRequest,
};

/// One UTF-8 source body captured at a portable logical label.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct TextDocumentInput {
    label: String,
    content: String,
}

impl TextDocumentInput {
    /// Preserve this UTF-8 body as exact bytes in the portable workspace DTO.
    fn into_workspace(self) -> PortableWorkspaceDocument {
        PortableWorkspaceDocument::new(self.label, self.content.into_bytes())
    }

    /// Preserve this UTF-8 body as exact bytes in the captured compilation DTO.
    fn into_compilation(self) -> CompilationDocument {
        CompilationDocument::new(self.label, self.content.into_bytes())
    }
}

/// Complete path-free workspace source family.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PortableWorkspaceInput {
    root_document: String,
    documents: Vec<TextDocumentInput>,
}

impl PortableWorkspaceInput {
    /// Retain the distinguished root label and complete document order.
    fn into_api(self) -> PortableWorkspaceRequest {
        PortableWorkspaceRequest::new(
            self.root_document,
            self.documents
                .into_iter()
                .map(TextDocumentInput::into_workspace)
                .collect(),
        )
    }
}

/// Input for strict validation against one selected generated contract.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ContractValidationInput {
    schema_id: String,
    document: String,
}

impl ContractValidationInput {
    /// Preserve the selected schema and exact UTF-8 document in the dispatcher request.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::ValidateContractDocument(ContractDocument::new(
            self.schema_id,
            self.document.into_bytes(),
        ))
    }
}

/// Exact locator and pre-signed authority envelope for one resident read.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ResidentObservationInput {
    /// Immutable storage reference for the retained operation handle.
    operation_handle_record_ref: String,
    /// SHA-256 of the unsigned canonical operation-handle preimage.
    operation_handle_sha256: String,
    /// Exact pre-signed common request as canonical JSON text.
    signed_common_request: String,
}

impl ResidentObservationInput {
    /// Preserve the caller's exact locator and signed canonical JSON bytes.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::ResidentOperationObservation(ResidentOperationObservationRequest::new(
            OperationHandleLocator::new(
                self.operation_handle_record_ref,
                self.operation_handle_sha256,
            ),
            self.signed_common_request.into_bytes(),
        ))
    }
}

/// Pre-signed request for one durable root Invoke start.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ResidentInvokeStartInput {
    /// Exact pre-signed Invoke request as canonical JSON text.
    signed_common_request: String,
}

impl ResidentInvokeStartInput {
    /// Preserve the caller's exact canonical request bytes.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::ResidentInvokeStart(ResidentInvokeStartRequest::new(
            self.signed_common_request.into_bytes(),
        ))
    }
}

/// Pre-signed request for one already-selected Consult start.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ResidentConsultStartInput {
    /// Exact pre-signed Consult request as canonical JSON text.
    signed_common_request: String,
}

impl ResidentConsultStartInput {
    /// Preserve the caller's exact canonical request bytes.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::ResidentConsultStart(ResidentConsultStartRequest::new(
            self.signed_common_request.into_bytes(),
        ))
    }
}

/// Exact locator and pre-signed request for one bounded resident continuation.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ResidentOperationContinuationInput {
    /// Immutable storage reference for the retained operation handle.
    operation_handle_record_ref: String,
    /// SHA-256 of the unsigned canonical operation-handle preimage.
    operation_handle_sha256: String,
    /// Exact pre-signed continuation or Cancel request as canonical JSON text.
    signed_common_request: String,
}

impl ResidentOperationContinuationInput {
    fn into_parts(self) -> (OperationHandleLocator, Vec<u8>) {
        (
            OperationHandleLocator::new(
                self.operation_handle_record_ref,
                self.operation_handle_sha256,
            ),
            self.signed_common_request.into_bytes(),
        )
    }

    /// Preserve one Invoke continuation without selecting an implementation owner.
    pub(super) fn into_invoke_command(self) -> CommandRequest {
        let (locator, signed_common_request) = self.into_parts();
        CommandRequest::ResidentInvokeAdvance(ResidentInvokeAdvanceRequest::new(
            locator,
            signed_common_request,
        ))
    }

    /// Preserve one Consult continuation without selecting its completion policy.
    pub(super) fn into_consult_command(self) -> CommandRequest {
        let (locator, signed_common_request) = self.into_parts();
        CommandRequest::ResidentConsultAdvance(ResidentConsultAdvanceRequest::new(
            locator,
            signed_common_request,
        ))
    }

    /// Preserve one Delegate continuation without selecting a route.
    pub(super) fn into_delegate_command(self) -> CommandRequest {
        let (locator, signed_common_request) = self.into_parts();
        CommandRequest::ResidentDelegateAdvance(ResidentDelegateAdvanceRequest::new(
            locator,
            signed_common_request,
        ))
    }

    /// Preserve one Cancel request without claiming terminal cancellation.
    pub(super) fn into_cancel_command(self) -> CommandRequest {
        let (locator, signed_common_request) = self.into_parts();
        CommandRequest::ResidentOperationCancellation(ResidentOperationCancellationRequest::new(
            locator,
            signed_common_request,
        ))
    }
}

/// Exact parent and pre-signed request for one pairwise Delegate start.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ResidentDelegateStartInput {
    /// Immutable storage reference for the exact parent operation handle.
    parent_operation_handle_record_ref: String,
    /// SHA-256 of the unsigned canonical parent operation-handle preimage.
    parent_operation_handle_sha256: String,
    /// Exact pre-signed Delegate request as canonical JSON text.
    signed_common_request: String,
}

impl ResidentDelegateStartInput {
    /// Preserve the exact parent coordinate and signed Delegate request.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::ResidentDelegateStart(ResidentDelegateStartRequest::new(
            OperationHandleLocator::new(
                self.parent_operation_handle_record_ref,
                self.parent_operation_handle_sha256,
            ),
            self.signed_common_request.into_bytes(),
        ))
    }
}

/// Input for one path-free portable workspace compilation.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PortableCompileInput {
    workspace: PortableWorkspaceInput,
}

impl PortableCompileInput {
    /// Consume the complete source family into its path-free dispatcher request.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::PortableWorkspaceCompilation(self.workspace.into_api())
    }
}

/// Complete predecessor/current workspace pair.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PortableComparisonInput {
    prior: PortableWorkspaceInput,
    current: PortableWorkspaceInput,
}

impl PortableComparisonInput {
    /// Preserve the predecessor/current ordering for every comparison request.
    fn into_api(self) -> (PortableWorkspaceRequest, PortableWorkspaceRequest) {
        (self.prior.into_api(), self.current.into_api())
    }

    /// Preserve the predecessor/current ordering in the comparison request.
    pub(super) fn into_command(self) -> CommandRequest {
        let (prior, current) = self.into_api();
        CommandRequest::PortableWorkspaceComparison { prior, current }
    }
}

/// One bounded page request over a stateless workspace comparison.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PortablePageInput {
    prior: PortableWorkspaceInput,
    current: PortableWorkspaceInput,
    offset: u32,
    limit: u16,
}

impl PortablePageInput {
    /// Build the bounded change-root query for this stateless comparison.
    pub(super) fn into_roots_command(self) -> CommandRequest {
        CommandRequest::PortableWorkspaceChangeRoots {
            prior: self.prior.into_api(),
            current: self.current.into_api(),
            offset: self.offset,
            limit: self.limit,
        }
    }

    /// Build the bounded impact query for this stateless comparison.
    pub(super) fn into_impacts_command(self) -> CommandRequest {
        CommandRequest::PortableWorkspaceImpacts {
            prior: self.prior.into_api(),
            current: self.current.into_api(),
            offset: self.offset,
            limit: self.limit,
        }
    }
}

/// Closed MCP spelling of [`PortableWorkspaceSubjectSelector`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(super) enum PortableSubjectInput {
    /// Select one retained logical source document.
    Source {
        /// Receipt-local source identity.
        source_id: String,
    },
    /// Select one admitted semantic source component.
    Component {
        /// Component identity.
        component_id: String,
    },
    /// Select one module in the admitted release closure.
    Module {
        /// Module identity.
        module_id: String,
    },
    /// Select one canonically owned declaration.
    Declaration {
        /// Canonical owner module.
        owner_module_id: String,
        /// Namespace containing the declaration.
        namespace_id: String,
        /// Declaration identity local to the namespace.
        declaration_id: String,
    },
    /// Select one resolved import path without flattening its compound coordinate.
    Import {
        /// Module declaring the import.
        importer_module_id: String,
        /// Selected upstream module.
        upstream_module_id: String,
        /// Selected upstream release.
        upstream_release_id: String,
        /// Named upstream export.
        export_id: String,
        /// Namespace selected from the upstream export.
        source_namespace_id: String,
        /// Namespace bound in the importing module.
        local_namespace_id: String,
    },
    /// Select one applied extension without flattening owner and target identities.
    Extension {
        /// Module that owns the extension.
        owner_module_id: String,
        /// Namespace that owns the extension rule.
        extension_namespace_id: String,
        /// Namespace of the extended declaration.
        target_namespace_id: String,
        /// Identity of the extended declaration.
        target_declaration_id: String,
    },
    /// Select the effective policy of one declaration.
    Policy {
        /// Canonical owner module.
        owner_module_id: String,
        /// Namespace containing the declaration.
        namespace_id: String,
        /// Declaration identity local to the namespace.
        declaration_id: String,
    },
    /// Select the complete responsibility closure.
    ResponsibilityClosure,
    /// Select the complete optional organizational-translation closure.
    OrganizationalTranslationClosure,
    /// Select the complete explicit experience-composition closure.
    ExperienceComposition,
    /// Select one logical or release-bound resource.
    Resource {
        /// Module selecting the resource.
        owner_module_id: String,
        /// Workspace-local resource identity.
        resource_id: String,
    },
    /// Select the exact resolution lock.
    ResolutionLock,
    /// Select the exact universe lock.
    UniverseLock,
    /// Select the complete workspace semantic IR.
    Workspace {
        /// Principal-relative workspace identity.
        workspace_id: String,
    },
}

impl From<PortableSubjectInput> for PortableWorkspaceSubjectSelector {
    fn from(input: PortableSubjectInput) -> Self {
        match input {
            PortableSubjectInput::Source { source_id } => Self::Source { source_id },
            PortableSubjectInput::Component { component_id } => Self::Component { component_id },
            PortableSubjectInput::Module { module_id } => Self::Module { module_id },
            PortableSubjectInput::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            PortableSubjectInput::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            },
            PortableSubjectInput::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            },
            PortableSubjectInput::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            PortableSubjectInput::ResponsibilityClosure => Self::ResponsibilityClosure,
            PortableSubjectInput::OrganizationalTranslationClosure => {
                Self::OrganizationalTranslationClosure
            }
            PortableSubjectInput::ExperienceComposition => Self::ExperienceComposition,
            PortableSubjectInput::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource {
                owner_module_id,
                resource_id,
            },
            PortableSubjectInput::ResolutionLock => Self::ResolutionLock,
            PortableSubjectInput::UniverseLock => Self::UniverseLock,
            PortableSubjectInput::Workspace { workspace_id } => Self::Workspace { workspace_id },
        }
    }
}

/// Stateless causal-explanation request.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PortableExplanationInput {
    prior: PortableWorkspaceInput,
    current: PortableWorkspaceInput,
    subject: PortableSubjectInput,
}

impl PortableExplanationInput {
    /// Forward the closed selector to the API-owned admission boundary.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::PortableWorkspaceExplanation(DispatcherExplanationRequest::new(
            self.prior.into_api(),
            self.current.into_api(),
            self.subject.into(),
        ))
    }
}

/// Complete private input roles for one captured compiler invocation.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct OneShotPrivateInput {
    capabilities: TextDocumentInput,
    machine_binding: TextDocumentInput,
    distribution_lock: TextDocumentInput,
    lifecycle_guidance: Vec<TextDocumentInput>,
}

impl OneShotPrivateInput {
    /// Bind each private source body to its distinct API-owned role.
    fn into_api(self) -> OneShotPrivateInputs {
        OneShotPrivateInputs::new(
            self.capabilities.into_compilation(),
            self.machine_binding.into_compilation(),
            self.distribution_lock.into_compilation(),
            self.lifecycle_guidance
                .into_iter()
                .map(TextDocumentInput::into_compilation)
                .collect(),
        )
    }
}

/// Candidate workspace and private sources; the baseline retains Package ownership.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ConfigurationReviewCandidateInput {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: OneShotPrivateInput,
    workspace: PortableWorkspaceInput,
}

impl ConfigurationReviewCandidateInput {
    /// Preserve the candidate roles without introducing a second Package input.
    fn with_baseline(self, baseline: OneShotCompilationInput) -> ConfigurationImprovementSources {
        let (workspace, profile) = baseline.into_effective_profile();
        ConfigurationImprovementSources::new(
            workspace,
            profile,
            ConfigurationImprovementProposalRequest::new(
                self.project_id,
                self.execution_workspace_id,
                self.machine_id,
                self.workspace.into_api(),
                self.private_inputs.into_api(),
            ),
        )
    }
}

/// One explicit review disposition, not a request to apply source edits.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum ConfigurationReviewDispositionInput {
    Accepted,
    Rejected,
    Cancelled,
    Stale,
}

impl From<ConfigurationReviewDispositionInput> for ConfigurationImprovementDisposition {
    fn from(value: ConfigurationReviewDispositionInput) -> Self {
        match value {
            ConfigurationReviewDispositionInput::Accepted => Self::Accepted,
            ConfigurationReviewDispositionInput::Rejected => Self::Rejected,
            ConfigurationReviewDispositionInput::Cancelled => Self::Cancelled,
            ConfigurationReviewDispositionInput::Stale => Self::Stale,
        }
    }
}

/// Reconstruct a review on every request; finishing checks the exact presented summary.
#[derive(Debug, Deserialize, JsonSchema)]
// Every tagged variant is an object; MCP also requires that type at the root.
#[schemars(extend("type" = "object"))]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum ConfigurationReviewInput {
    Preview {
        baseline: OneShotCompilationInput,
        candidate: ConfigurationReviewCandidateInput,
    },
    /// Return one bounded source-evidence page for a checked review subject.
    ///
    /// MCP reconstructs the review for each call and therefore cannot accept
    /// the SDK's retained, owner-bound continuation cursor.
    Inspect {
        baseline: OneShotCompilationInput,
        candidate: ConfigurationReviewCandidateInput,
        /// Complete preview summary returned by the earlier request.
        #[schemars(with = "ConfigurationImprovementPreviewSummary")]
        summary: serde_json::Map<String, serde_json::Value>,
        /// Compiler subject whose causal and source evidence is requested.
        subject: PortableSubjectInput,
        /// Maximum direct proof inputs returned; zero selects the SDK default.
        input_limit: u16,
        /// Maximum source candidates examined; zero selects the SDK default.
        maximum_inspected: u16,
        /// Maximum matching source occurrences returned; zero selects the SDK default.
        maximum_returned: u16,
    },
    Finish {
        baseline: OneShotCompilationInput,
        candidate: ConfigurationReviewCandidateInput,
        /// Complete earlier summary, kept as an MCP object until direct byte admission.
        #[schemars(with = "ConfigurationImprovementPreviewSummary")]
        summary: serde_json::Map<String, serde_json::Value>,
        disposition: ConfigurationReviewDispositionInput,
    },
}

impl ConfigurationReviewInput {
    /// Use the complete SDK-owned source pipeline through the shared dispatcher.
    ///
    /// # Errors
    ///
    /// Returns an internal MCP error only if the already parsed summary object
    /// cannot be encoded as the SDK-owned presented-summary bytes.
    pub(super) fn into_command(self) -> Result<CommandRequest, rmcp::ErrorData> {
        let command = match self {
            Self::Preview {
                baseline,
                candidate,
            } => ConfigurationImprovementCommand::PreviewSources(Box::new(
                candidate.with_baseline(baseline),
            )),
            Self::Inspect {
                baseline,
                candidate,
                summary,
                subject,
                input_limit,
                maximum_inspected,
                maximum_returned,
            } => ConfigurationImprovementCommand::InspectSources {
                sources: Box::new(candidate.with_baseline(baseline)),
                presented_summary: encode_review_summary(summary)?,
                subject: subject.into(),
                input_limit,
                maximum_inspected,
                maximum_returned,
            },
            Self::Finish {
                baseline,
                candidate,
                summary,
                disposition,
            } => ConfigurationImprovementCommand::FinishSources {
                sources: Box::new(candidate.with_baseline(baseline)),
                presented_summary: encode_review_summary(summary)?,
                disposition: disposition.into(),
            },
        };
        Ok(CommandRequest::ConfigurationImprovement(command))
    }
}

fn encode_review_summary(
    summary: serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<u8>, rmcp::ErrorData> {
    serde_json::to_vec(&summary).map_err(|_| {
        rmcp::ErrorData::internal_error(
            "Pale configuration-review summary could not be encoded",
            None,
        )
    })
}

/// Complete captured source family for one one-shot compilation.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct OneShotCompilationInput {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: OneShotPrivateInput,
    package_documents: Vec<TextDocumentInput>,
    workspace: PortableWorkspaceInput,
}

impl OneShotCompilationInput {
    /// Preserve role boundaries for an effective-profile request over an
    /// independently admitted portable workspace.
    fn into_effective_profile(
        self,
    ) -> (PortableWorkspaceRequest, EffectiveProfileCompilationRequest) {
        let Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
            package_documents,
            workspace,
        } = self;
        (
            workspace.into_api(),
            EffectiveProfileCompilationRequest::new(
                project_id,
                execution_workspace_id,
                machine_id,
                private_inputs.into_api(),
                package_documents
                    .into_iter()
                    .map(TextDocumentInput::into_compilation)
                    .collect(),
            ),
        )
    }

    /// Preserve identities and source-role partitions in the API request.
    fn into_api(self) -> OneShotCompilationRequest {
        OneShotCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
            self.package_documents
                .into_iter()
                .map(TextDocumentInput::into_compilation)
                .collect(),
            self.workspace.into_api(),
        )
    }

    /// Preserve every captured source role in one complete compilation request.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::OneShotCompilation(self.into_api())
    }
}

/// Closed MCP spelling of one effective-profile section.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(super) enum EffectiveProfileSectionInput {
    /// Declared responsibilities and use-case requirements.
    IntendedUseCase,
    /// Compiler-derived selections and policy ceilings.
    EffectiveConfiguration,
    /// Operation exposure and capability dispositions.
    CapabilitiesAndOperations,
    /// Separately qualified native realization.
    MachineRealization,
    /// Exact typed gaps and causal coordinates.
    GapsAndExplanations,
}

impl From<EffectiveProfileSectionInput> for EffectiveProfileSection {
    fn from(section: EffectiveProfileSectionInput) -> Self {
        match section {
            EffectiveProfileSectionInput::IntendedUseCase => Self::IntendedUseCase,
            EffectiveProfileSectionInput::EffectiveConfiguration => Self::EffectiveConfiguration,
            EffectiveProfileSectionInput::CapabilitiesAndOperations => {
                Self::CapabilitiesAndOperations
            }
            EffectiveProfileSectionInput::MachineRealization => Self::MachineRealization,
            EffectiveProfileSectionInput::GapsAndExplanations => Self::GapsAndExplanations,
        }
    }
}

/// Stateless effective-profile inspection over complete role-separated inputs.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct EffectiveProfileInspectionInput {
    compilation: OneShotCompilationInput,
    section: EffectiveProfileSectionInput,
    offset: u32,
    limit: u16,
}

impl EffectiveProfileInspectionInput {
    /// Bind the exact workspace owner and remaining source roles to one page.
    pub(super) fn into_command(self) -> CommandRequest {
        let (workspace, profile) = self.compilation.into_effective_profile();
        CommandRequest::EffectiveProfileInspection(DispatcherProfileInspectionRequest::new(
            workspace,
            profile,
            EffectiveProfilePageRequest::new(self.section.into(), self.offset, self.limit),
        ))
    }
}

/// Resolve a page's opaque explanation handle against freshly admitted sources.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct EffectiveProfileExplanationInput {
    compilation: OneShotCompilationInput,
    #[schemars(with = "String")]
    handle: EffectiveProfileExplanationHandle,
}

impl EffectiveProfileExplanationInput {
    /// Leave handle admission and full-coordinate lookup with their SDK owners.
    pub(super) fn into_command(self) -> CommandRequest {
        let (workspace, profile) = self.compilation.into_effective_profile();
        CommandRequest::EffectiveProfileExplanation(DispatcherProfileExplanationRequest::new(
            workspace,
            profile,
            self.handle,
        ))
    }
}

/// Closed spelling of one effect-free planning operation.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum OneShotPlanKindInput {
    /// Reconcile desired surfaces with prior ownership when present.
    Apply,
    /// Adopt matching pre-existing generated targets on first ownership.
    Adopt,
    /// Remove only surfaces proven by prior ownership state.
    Remove,
}

impl From<OneShotPlanKindInput> for OneShotPlanKind {
    fn from(kind: OneShotPlanKindInput) -> Self {
        match kind {
            OneShotPlanKindInput::Apply => Self::Apply,
            OneShotPlanKindInput::Adopt => Self::Adopt,
            OneShotPlanKindInput::Remove => Self::Remove,
        }
    }
}

/// Claimed prior Pale materialization state.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(
    deny_unknown_fields,
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(super) enum OneShotPriorInput {
    /// Claim that no prior Pale materialization exists.
    Unowned,
    /// Carry exact canonical bytes for the claimed prior Pale state.
    Owned {
        /// Canonical prior-state document retained for API admission.
        canonical_state: String,
    },
}

impl OneShotPriorInput {
    /// Preserve the prior-state claim without treating it as an observation.
    fn into_api(self) -> OneShotPriorMaterialization {
        match self {
            Self::Unowned => OneShotPriorMaterialization::Unowned,
            Self::Owned { canonical_state } => OneShotPriorMaterialization::Owned {
                canonical_state: canonical_state.into_bytes(),
            },
        }
    }
}

/// Stateless one-shot observation-target request.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct OneShotObservationInput {
    compilation: OneShotCompilationInput,
    repository_id: String,
    kind: OneShotPlanKindInput,
    prior: OneShotPriorInput,
}

impl OneShotObservationInput {
    /// Bind the captured compilation to one repository and planning operation.
    pub(super) fn into_command(self) -> CommandRequest {
        CommandRequest::OneShotObservationTargets {
            compilation: self.compilation.into_api(),
            repository_id: self.repository_id,
            kind: self.kind.into(),
            prior: self.prior.into_api(),
        }
    }
}

/// Claimed pre-mutation observation for one plan target.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(
    deny_unknown_fields,
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum OneShotBaselineObservationInput {
    Absent,
    Present { sha256: String, unix_mode: u32 },
}

impl OneShotBaselineObservationInput {
    /// Preserve the caller's claimed state without performing native observation.
    fn into_api(self) -> OneShotBaselineObservation {
        match self {
            Self::Absent => OneShotBaselineObservation::Absent,
            Self::Present { sha256, unix_mode } => {
                OneShotBaselineObservation::Present { sha256, unix_mode }
            }
        }
    }
}

/// One claimed observation bound to a repository-relative target.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct OneShotBaselineInput {
    relative_path: String,
    observation: OneShotBaselineObservationInput,
}

impl OneShotBaselineInput {
    /// Bind one claimed observation to its repository-relative target.
    fn into_api(self) -> OneShotBaselineEntry {
        OneShotBaselineEntry::new(self.relative_path, self.observation.into_api())
    }
}

/// Stateless effect-free one-shot planning request.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct OneShotPlanningInput {
    compilation: OneShotCompilationInput,
    repository_id: String,
    kind: OneShotPlanKindInput,
    prior: OneShotPriorInput,
    baseline: Vec<OneShotBaselineInput>,
}

impl OneShotPlanningInput {
    /// Preserve caller observations in one stateless effect-free planning request.
    pub(super) fn into_command(self) -> CommandRequest {
        let kind = self.kind.into();
        let request = OneShotPlanningRequest::new(
            self.repository_id,
            kind,
            self.prior.into_api(),
            self.baseline
                .into_iter()
                .map(OneShotBaselineInput::into_api)
                .collect(),
        );
        CommandRequest::OneShotPlanning {
            compilation: self.compilation.into_api(),
            request,
        }
    }
}
