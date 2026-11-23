//! Closed MCP tool inventory over the typed command dispatcher.

use rmcp::handler::server::tool::schema_for_input;
use rmcp::model::{CallToolResult, ErrorData as McpError, JsonObject, Tool, ToolAnnotations};
use rmcp::schemars::{self, JsonSchema};
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde::Serialize;
use std::fmt::Write as _;
use std::sync::Arc;

use pale_cli::command::{
    ApiCapabilityId, CommandOutcome, CommandRequest, ConfigurationImprovementOutcome,
    ConfigurationImprovementReviewResult, EffectiveProfileExplanation, EffectiveProfilePage,
    NativeActionDescriptor, NativeActionExecution, NativeActionPayloadRequest,
    OneShotCompilationSummary, OneShotObservationTargets, OneShotPlanCandidate,
    PortableWorkspaceChangeRootPage, PortableWorkspaceComparisonSummary,
    PortableWorkspaceCompilationSummary, PortableWorkspaceExplanation, PortableWorkspaceImpactPage,
    ResidentConsultAdvanceOutcome, ResidentDelegateAdvanceOutcome, ResidentInvokeAdvanceOutcome,
    ResidentOperationCancellationOutcome, ResidentOperationObservation,
};

use crate::{projection, stage, wire, ProcessResult};

/// API capabilities implemented by the registry-derived native-action tools.
pub(super) const NATIVE_ACTION_CAPABILITY_IDS: &[ApiCapabilityId] = &[
    ApiCapabilityId::NativeActionCatalog,
    ApiCapabilityId::NativeActionPayloadInvocation,
];

/// One stateless façade operation deliberately exposed through MCP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FacadeTool {
    /// Project [`CommandRequest::ValidateContractDocument`].
    ValidateContract,
    /// Project [`CommandRequest::PortableWorkspaceCompilation`].
    CompilePortableWorkspace,
    /// Project [`CommandRequest::PortableWorkspaceComparison`].
    ComparePortableWorkspaces,
    /// Project [`CommandRequest::PortableWorkspaceChangeRoots`].
    ListPortableWorkspaceChangeRoots,
    /// Project [`CommandRequest::PortableWorkspaceImpacts`].
    ListPortableWorkspaceImpacts,
    /// Project [`CommandRequest::PortableWorkspaceExplanation`].
    ExplainPortableWorkspaceSubject,
    /// Project [`CommandRequest::EffectiveProfileInspection`].
    InspectEffectiveProfile,
    /// Resolve one full-coordinate-bound profile explanation through the dispatcher.
    ExplainEffectiveProfileEntry,
    /// Reconstruct a configuration review and optionally check a presented summary before finishing.
    ReviewConfiguration,
    /// Project [`CommandRequest::OneShotCompilation`].
    CompileOneShot,
    /// Project [`CommandRequest::OneShotObservationTargets`].
    ListOneShotObservationTargets,
    /// Project [`CommandRequest::OneShotPlanning`].
    PlanOneShot,
    /// Project [`CommandRequest::ResidentOperationObservation`].
    ObserveResidentOperation,
    /// Start one durable root Invoke.
    StartResidentInvoke,
    /// Advance one retained Invoke through its selected owner.
    AdvanceResidentInvoke,
    /// Start one already-selected durable Consult.
    StartResidentConsult,
    /// Advance one Consult to its next honest boundary.
    AdvanceResidentConsult,
    /// Start one pairwise Delegate child under its exact parent.
    StartResidentDelegate,
    /// Advance one Delegate through its already-selected dispatch owner.
    AdvanceResidentDelegate,
    /// Send one resident Cancel request.
    CancelResidentOperation,
}

/// One route from an MCP tool to the shared command dispatcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ToolRoute {
    /// One fixed façade operation.
    Facade(FacadeTool),
    /// One registry-owned action selected by its MCP tool name.
    NativeAction {
        /// Compiled action identity bound to the advertised payload schema.
        action_id: String,
    },
}

/// One immutable registration in the closed MCP tool inventory.
pub(super) struct ToolRegistration {
    /// Dispatcher route selected by this registration.
    pub(super) route: ToolRoute,
    /// MCP descriptor whose schema is derived from the matching wire input.
    pub(super) descriptor: Tool,
}

impl FacadeTool {
    /// Complete closed route inventory used for both registration and parity.
    const ALL: &[Self] = &[
        Self::ValidateContract,
        Self::CompilePortableWorkspace,
        Self::ComparePortableWorkspaces,
        Self::ListPortableWorkspaceChangeRoots,
        Self::ListPortableWorkspaceImpacts,
        Self::ExplainPortableWorkspaceSubject,
        Self::InspectEffectiveProfile,
        Self::ExplainEffectiveProfileEntry,
        Self::ReviewConfiguration,
        Self::CompileOneShot,
        Self::ListOneShotObservationTargets,
        Self::PlanOneShot,
        Self::ObserveResidentOperation,
        Self::StartResidentInvoke,
        Self::AdvanceResidentInvoke,
        Self::StartResidentConsult,
        Self::AdvanceResidentConsult,
        Self::StartResidentDelegate,
        Self::AdvanceResidentDelegate,
        Self::CancelResidentOperation,
    ];

    /// Borrow the complete route inventory without reconstructing tool names.
    pub(super) const fn all() -> &'static [Self] {
        Self::ALL
    }

    /// API-owned capabilities exercised by this exact transport route.
    pub(super) const fn capability_ids(self) -> &'static [ApiCapabilityId] {
        use ApiCapabilityId::{
            ContractDocumentValidation, EffectiveProfileCompilation, EffectiveProfileInspection,
            OneShotCompilation, OneShotCompilationSummary, OneShotObservationTargets,
            OneShotPlanning, PortableWorkspaceChangeRoots, PortableWorkspaceComparison,
            PortableWorkspaceCompilation, PortableWorkspaceExplanation, PortableWorkspaceImpacts,
        };

        match self {
            Self::ValidateContract => &[ContractDocumentValidation],
            Self::CompilePortableWorkspace => &[PortableWorkspaceCompilation],
            Self::ComparePortableWorkspaces => &[PortableWorkspaceComparison],
            Self::ListPortableWorkspaceChangeRoots => &[PortableWorkspaceChangeRoots],
            Self::ListPortableWorkspaceImpacts => &[PortableWorkspaceImpacts],
            Self::ExplainPortableWorkspaceSubject => &[PortableWorkspaceExplanation],
            Self::InspectEffectiveProfile => {
                &[EffectiveProfileCompilation, EffectiveProfileInspection]
            }
            Self::ExplainEffectiveProfileEntry => &[ApiCapabilityId::EffectiveProfileExplanation],
            Self::ReviewConfiguration => &[ApiCapabilityId::ConfigurationImprovement],
            Self::CompileOneShot => &[OneShotCompilation, OneShotCompilationSummary],
            Self::ListOneShotObservationTargets => &[OneShotObservationTargets],
            Self::PlanOneShot => &[OneShotPlanning],
            Self::ObserveResidentOperation => &[ApiCapabilityId::ResidentOperationObservation],
            Self::StartResidentInvoke => &[ApiCapabilityId::ResidentInvokeStart],
            Self::AdvanceResidentInvoke => &[ApiCapabilityId::ResidentInvokeAdvance],
            Self::StartResidentConsult => &[ApiCapabilityId::ResidentConsultStart],
            Self::AdvanceResidentConsult => &[ApiCapabilityId::ResidentConsultAdvance],
            Self::StartResidentDelegate => &[ApiCapabilityId::ResidentDelegateStart],
            Self::AdvanceResidentDelegate => &[ApiCapabilityId::ResidentDelegateAdvance],
            Self::CancelResidentOperation => &[ApiCapabilityId::ResidentOperationCancellation],
        }
    }

    /// Short label that remains distinct when clients show several tools together.
    const fn title(self) -> &'static str {
        match self {
            Self::ValidateContract => "Validate contract",
            Self::CompilePortableWorkspace => "Compile workspace",
            Self::ComparePortableWorkspaces => "Compare workspaces",
            Self::ListPortableWorkspaceChangeRoots => "List change causes",
            Self::ListPortableWorkspaceImpacts => "List affected facts",
            Self::ExplainPortableWorkspaceSubject => "Explain workspace subject",
            Self::InspectEffectiveProfile => "Inspect effective profile",
            Self::ExplainEffectiveProfileEntry => "Explain profile entry",
            Self::ReviewConfiguration => "Review configuration",
            Self::CompileOneShot => "Compile captured configuration",
            Self::ListOneShotObservationTargets => "List observation targets",
            Self::PlanOneShot => "Plan materialization",
            Self::ObserveResidentOperation => "Observe resident operation",
            Self::StartResidentInvoke => "Start resident Invoke",
            Self::AdvanceResidentInvoke => "Advance resident Invoke",
            Self::StartResidentConsult => "Start resident Consult",
            Self::AdvanceResidentConsult => "Advance resident Consult",
            Self::StartResidentDelegate => "Start resident Delegate",
            Self::AdvanceResidentDelegate => "Advance resident Delegate",
            Self::CancelResidentOperation => "Cancel resident operation",
        }
    }

    /// Describe the observable effect boundary of this exact tool route.
    fn annotations(self) -> ToolAnnotations {
        match self {
            Self::StartResidentInvoke
            | Self::StartResidentConsult
            | Self::StartResidentDelegate => resident_start_annotations(),
            Self::AdvanceResidentInvoke
            | Self::AdvanceResidentConsult
            | Self::AdvanceResidentDelegate => resident_advance_annotations(),
            Self::CancelResidentOperation => resident_cancellation_annotations(),
            _ => effect_free_annotations(),
        }
    }

    /// Attach only schemas already owned by the SDK output type.
    ///
    /// A route is omitted when one fixed result schema would be untrue. For
    /// example, contract validation returns the document shape selected by the
    /// caller's `schemaId`. Advertising a broad object schema would weaken,
    /// not improve, discovery.
    fn with_supported_output_schema(self, tool: Tool) -> Tool {
        match self {
            Self::CompilePortableWorkspace => {
                tool.with_output_schema::<PortableWorkspaceCompilationSummary>()
            }
            Self::ComparePortableWorkspaces => {
                tool.with_output_schema::<PortableWorkspaceComparisonSummary>()
            }
            Self::ListPortableWorkspaceChangeRoots => {
                tool.with_output_schema::<PortableWorkspaceChangeRootPage>()
            }
            Self::ListPortableWorkspaceImpacts => {
                tool.with_output_schema::<PortableWorkspaceImpactPage>()
            }
            Self::ExplainPortableWorkspaceSubject => {
                tool.with_output_schema::<PortableWorkspaceExplanation>()
            }
            Self::InspectEffectiveProfile => tool.with_output_schema::<EffectiveProfilePage>(),
            Self::ExplainEffectiveProfileEntry => {
                tool.with_output_schema::<EffectiveProfileExplanationOutput>()
            }
            Self::CompileOneShot => tool.with_output_schema::<OneShotCompilationSummary>(),
            Self::ListOneShotObservationTargets => {
                tool.with_output_schema::<OneShotObservationTargets>()
            }
            Self::PlanOneShot => tool.with_output_schema::<OneShotPlanCandidate>(),
            Self::ReviewConfiguration => {
                tool.with_output_schema::<ConfigurationImprovementReviewResult>()
            }
            Self::ObserveResidentOperation => {
                tool.with_output_schema::<ResidentOperationObservation>()
            }
            Self::StartResidentInvoke
            | Self::StartResidentConsult
            | Self::StartResidentDelegate => {
                tool.with_output_schema::<ResidentOperationObservation>()
            }
            Self::AdvanceResidentInvoke => {
                tool.with_output_schema::<ResidentInvokeAdvanceOutcome>()
            }
            Self::AdvanceResidentConsult => {
                tool.with_output_schema::<ResidentConsultAdvanceOutcome>()
            }
            Self::AdvanceResidentDelegate => {
                tool.with_output_schema::<ResidentDelegateAdvanceOutcome>()
            }
            Self::CancelResidentOperation => {
                tool.with_output_schema::<ResidentOperationCancellationOutcome>()
            }
            Self::ValidateContract => tool,
        }
    }

    /// Construct the complete stable MCP façade-tool set in lexical order.
    ///
    /// # Errors
    ///
    /// Returns an error when the SDK cannot derive an object schema for any
    /// closed wire input.
    pub(super) fn registrations(
        resident_observation_attached: bool,
        resident_operation_attached: bool,
    ) -> ProcessResult<Vec<ToolRegistration>> {
        let mut registrations = Self::ALL
            .iter()
            .copied()
            .filter(|route| {
                (resident_observation_attached || !matches!(route, Self::ObserveResidentOperation))
                    && (resident_operation_attached
                        || !matches!(
                            route,
                            Self::StartResidentInvoke
                                | Self::AdvanceResidentInvoke
                                | Self::StartResidentConsult
                                | Self::AdvanceResidentConsult
                                | Self::StartResidentDelegate
                                | Self::AdvanceResidentDelegate
                                | Self::CancelResidentOperation
                        ))
            })
            .map(Self::registration)
            .collect::<ProcessResult<Vec<_>>>()?;
        registrations.sort_by(|left, right| left.descriptor.name.cmp(&right.descriptor.name));
        Ok(registrations)
    }

    /// Derive the wire descriptor paired with this closed dispatcher route.
    fn registration(self) -> ProcessResult<ToolRegistration> {
        match self {
            Self::ValidateContract => registration::<wire::ContractValidationInput>(
                self,
                "validate_contract",
                "Check whether JSON text is valid against a selected schema bundled with Pale, before you pass it to another tool or write it to a file.",
            ),
            Self::CompilePortableWorkspace => registration::<wire::PortableCompileInput>(
                self,
                "compile_portable_workspace",
                "Check that the supplied workspace documents compile, and return the compiled result, without reading local files. Use it first after editing a workspace.",
            ),
            Self::ComparePortableWorkspaces => registration::<wire::PortableComparisonInput>(
                self,
                "compare_portable_workspaces",
                "See what a proposed workspace edit changes in meaning: compile the supplied before-and-after documents and compare them.",
            ),
            Self::ListPortableWorkspaceChangeRoots => registration::<wire::PortablePageInput>(
                self,
                "list_portable_workspace_change_roots",
                "Find what caused a proposed workspace change: compile the supplied comparison and return one page of its direct causes.",
            ),
            Self::ListPortableWorkspaceImpacts => registration::<wire::PortablePageInput>(
                self,
                "list_portable_workspace_impacts",
                "See what a proposed workspace change affects: compile the supplied comparison and return one page of affected and unaffected facts.",
            ),
            Self::ExplainPortableWorkspaceSubject => {
                registration::<wire::PortableExplanationInput>(
                    self,
                    "explain_portable_workspace_subject",
                    "Explain why one subject changed, or did not, in the supplied workspace comparison.",
                )
            }
            Self::InspectEffectiveProfile => registration::<wire::EffectiveProfileInspectionInput>(
                self,
                "inspect_effective_profile",
                "See what applies to a repository or agent: compile the supplied configuration and return one page of its effective profile. This does not observe machine readiness.",
            ),
            Self::ReviewConfiguration => registration::<wire::ConfigurationReviewInput>(
                self,
                "review_configuration",
                "Review a proposed configuration change before anyone edits files: preview it, inspect one checked subject's causal and source evidence, or finish the checked review. This tool does not edit files.",
            ),
            Self::ExplainEffectiveProfileEntry => {
                registration::<wire::EffectiveProfileExplanationInput>(
                    self,
                    "explain_effective_profile_entry",
                    "Explain why an entry is in the effective profile, using the supplied sources. Return no explanation if its handle belongs to a different compiled profile, including its private instance.",
                )
            }
            Self::CompileOneShot => registration::<wire::OneShotCompilationInput>(
                self,
                "compile_one_shot",
                "See what Pale would generate for a repository: compile the supplied Package, private configuration and workspace documents without changing files or running native actions.",
            ),
            Self::ListOneShotObservationTargets => registration::<wire::OneShotObservationInput>(
                self,
                "list_one_shot_observation_targets",
                "List the repository paths whose current state you must supply before Pale can plan the files it would generate.",
            ),
            Self::PlanOneShot => registration::<wire::OneShotPlanningInput>(
                self,
                "plan_one_shot",
                "Plan the files Pale would generate in a repository from the supplied observations of its current state. This does not apply the plan; applying it is a separate, permitted step.",
            ),
            Self::ObserveResidentOperation => registration::<wire::ResidentObservationInput>(
                self,
                "observe_resident_operation",
                "Check what happened to one retained operation: read its signed records through the caller-provisioned resident attachment.",
            ),
            Self::StartResidentInvoke => registration::<wire::ResidentInvokeStartInput>(
                self,
                "start_resident_invoke",
                "Start one durable root Invoke of a declared operation and return its handle before dispatch; advance it with advance_resident_invoke.",
            ),
            Self::AdvanceResidentInvoke => {
                registration::<wire::ResidentOperationContinuationInput>(
                    self,
                    "advance_resident_invoke",
                    "Advance one retained Invoke by one bounded turn through its application-selected implementation owner.",
                )
            }
            Self::StartResidentConsult => registration::<wire::ResidentConsultStartInput>(
                self,
                "start_resident_consult",
                "Ask for a continue-or-stop judgment: start one already-selected Consult and return its durable handle with the retained compiled-policy reference before dispatch.",
            ),
            Self::AdvanceResidentConsult => {
                registration::<wire::ResidentOperationContinuationInput>(
                    self,
                    "advance_resident_consult",
                    "Advance one retained Consult by one bounded turn under its exact retained compiled policy. The caller cannot choose or default Continue or Stop.",
                )
            }
            Self::StartResidentDelegate => registration::<wire::ResidentDelegateStartInput>(
                self,
                "start_resident_delegate",
                "Hand one checked task to another agent system: start one authorized pairwise Delegate child and return its handle before dispatch.",
            ),
            Self::AdvanceResidentDelegate => {
                registration::<wire::ResidentOperationContinuationInput>(
                    self,
                    "advance_resident_delegate",
                    "Advance one retained Delegate only through its already-selected provider or A2A route.",
                )
            }
            Self::CancelResidentOperation => {
                registration::<wire::ResidentOperationContinuationInput>(
                    self,
                    "cancel_resident_operation",
                    "Send one Cancel request and return the resident owner's exact disposition. This does not prove terminal cancellation.",
                )
            }
        }
    }

    /// Translate an MCP object into one exact typed dispatcher request.
    ///
    /// # Errors
    ///
    /// Returns [`McpError`] only when the object violates its advertised MCP
    /// shape. Semantic identity admission occurs later through the dispatcher
    /// and therefore retains the API-owned [`pale_cli::command::DiagnosticV1`].
    pub(super) fn command(self, arguments: JsonObject) -> Result<CommandRequest, McpError> {
        match self {
            Self::ValidateContract => parse::<wire::ContractValidationInput>(arguments)
                .map(wire::ContractValidationInput::into_command),
            Self::CompilePortableWorkspace => parse::<wire::PortableCompileInput>(arguments)
                .map(wire::PortableCompileInput::into_command),
            Self::ComparePortableWorkspaces => parse::<wire::PortableComparisonInput>(arguments)
                .map(wire::PortableComparisonInput::into_command),
            Self::ListPortableWorkspaceChangeRoots => parse::<wire::PortablePageInput>(arguments)
                .map(wire::PortablePageInput::into_roots_command),
            Self::ListPortableWorkspaceImpacts => parse::<wire::PortablePageInput>(arguments)
                .map(wire::PortablePageInput::into_impacts_command),
            Self::ExplainPortableWorkspaceSubject => {
                parse::<wire::PortableExplanationInput>(arguments)
                    .map(wire::PortableExplanationInput::into_command)
            }
            Self::InspectEffectiveProfile => {
                parse::<wire::EffectiveProfileInspectionInput>(arguments)
                    .map(wire::EffectiveProfileInspectionInput::into_command)
            }
            Self::ReviewConfiguration => parse::<wire::ConfigurationReviewInput>(arguments)
                .and_then(wire::ConfigurationReviewInput::into_command),
            Self::ExplainEffectiveProfileEntry => {
                parse::<wire::EffectiveProfileExplanationInput>(arguments)
                    .map(wire::EffectiveProfileExplanationInput::into_command)
            }
            Self::CompileOneShot => parse::<wire::OneShotCompilationInput>(arguments)
                .map(wire::OneShotCompilationInput::into_command),
            Self::ListOneShotObservationTargets => {
                parse::<wire::OneShotObservationInput>(arguments)
                    .map(wire::OneShotObservationInput::into_command)
            }
            Self::PlanOneShot => parse::<wire::OneShotPlanningInput>(arguments)
                .map(wire::OneShotPlanningInput::into_command),
            Self::ObserveResidentOperation => parse::<wire::ResidentObservationInput>(arguments)
                .map(wire::ResidentObservationInput::into_command),
            Self::StartResidentInvoke => parse::<wire::ResidentInvokeStartInput>(arguments)
                .map(wire::ResidentInvokeStartInput::into_command),
            Self::AdvanceResidentInvoke => {
                parse::<wire::ResidentOperationContinuationInput>(arguments)
                    .map(wire::ResidentOperationContinuationInput::into_invoke_command)
            }
            Self::StartResidentConsult => parse::<wire::ResidentConsultStartInput>(arguments)
                .map(wire::ResidentConsultStartInput::into_command),
            Self::AdvanceResidentConsult => {
                parse::<wire::ResidentOperationContinuationInput>(arguments)
                    .map(wire::ResidentOperationContinuationInput::into_consult_command)
            }
            Self::StartResidentDelegate => parse::<wire::ResidentDelegateStartInput>(arguments)
                .map(wire::ResidentDelegateStartInput::into_command),
            Self::AdvanceResidentDelegate => {
                parse::<wire::ResidentOperationContinuationInput>(arguments)
                    .map(wire::ResidentOperationContinuationInput::into_delegate_command)
            }
            Self::CancelResidentOperation => {
                parse::<wire::ResidentOperationContinuationInput>(arguments)
                    .map(wire::ResidentOperationContinuationInput::into_cancel_command)
            }
        }
    }

    /// Project the matching API-owned outcome into MCP structured content.
    ///
    /// # Errors
    ///
    /// Returns [`McpError`] when the dispatcher returns a different closed
    /// outcome or API-owned JSON bytes cannot be decoded at the wire edge.
    pub(super) fn project(self, outcome: CommandOutcome) -> Result<CallToolResult, McpError> {
        match (self, outcome) {
            (Self::ValidateContract, CommandOutcome::ValidatedContractDocument(document)) => {
                projection::structured_bytes(document.canonical_bytes())
            }
            (
                Self::CompilePortableWorkspace,
                CommandOutcome::PortableWorkspaceCompilationSummary(summary),
            ) => projection::structured_bytes(summary.canonical_bytes()),
            (
                Self::ComparePortableWorkspaces,
                CommandOutcome::PortableWorkspaceComparisonSummary(summary),
            ) => projection::structured_bytes(summary.canonical_bytes()),
            (
                Self::ListPortableWorkspaceChangeRoots,
                CommandOutcome::PortableWorkspaceChangeRoots(page),
            ) => projection::structured_bytes(page.canonical_bytes()),
            (
                Self::ListPortableWorkspaceImpacts,
                CommandOutcome::PortableWorkspaceImpacts(page),
            ) => projection::structured_bytes(page.canonical_bytes()),
            (
                Self::ExplainPortableWorkspaceSubject,
                CommandOutcome::PortableWorkspaceExplanation(explanation),
            ) => projection::structured_bytes(explanation.canonical_bytes()),
            (Self::InspectEffectiveProfile, CommandOutcome::EffectiveProfilePage(page)) => {
                projection::structured_bytes(page.canonical_bytes())
            }
            (
                Self::ExplainEffectiveProfileEntry,
                CommandOutcome::EffectiveProfileExplanation(explanation),
            ) => serde_json::to_value(EffectiveProfileExplanationOutput { explanation })
                .map(CallToolResult::structured)
                .map_err(|_| {
                    McpError::internal_error("Pale profile explanation could not be encoded", None)
                }),
            (
                Self::ReviewConfiguration,
                CommandOutcome::ConfigurationImprovement(
                    ConfigurationImprovementOutcome::SourcePreviewed { preview, impacts },
                ),
            ) => project_configuration_review(ConfigurationImprovementReviewResult::Preview {
                summary: preview.summary().clone(),
                impacts,
            }),
            (
                Self::ReviewConfiguration,
                CommandOutcome::ConfigurationImprovement(
                    ConfigurationImprovementOutcome::SourceDetails {
                        summary,
                        explanation,
                        source,
                        source_page_state,
                    },
                ),
            ) => project_configuration_review(ConfigurationImprovementReviewResult::Details {
                summary: *summary,
                explanation,
                source,
                source_page_state,
            }),
            (
                Self::ReviewConfiguration,
                CommandOutcome::ConfigurationImprovement(
                    ConfigurationImprovementOutcome::Finished(terminal),
                ),
            ) => project_configuration_review(ConfigurationImprovementReviewResult::Finished {
                terminal: *terminal,
            }),
            (Self::CompileOneShot, CommandOutcome::OneShotCompilationSummary(summary)) => {
                projection::structured_bytes(summary.canonical_bytes())
            }
            (
                Self::ListOneShotObservationTargets,
                CommandOutcome::OneShotObservationTargets(targets),
            ) => projection::structured_bytes(targets.canonical_bytes()),
            (Self::PlanOneShot, CommandOutcome::OneShotPlanCandidate(candidate)) => {
                projection::structured_bytes(candidate.canonical_bytes())
            }
            (
                Self::ObserveResidentOperation,
                CommandOutcome::ResidentOperationObservation(observation),
            ) => serde_json::to_value(observation)
                .map(CallToolResult::structured)
                .map_err(|_| {
                    McpError::internal_error("Pale resident observation could not be encoded", None)
                }),
            (Self::StartResidentInvoke, CommandOutcome::ResidentInvokeStarted(observation))
            | (Self::StartResidentConsult, CommandOutcome::ResidentConsultStarted(observation))
            | (Self::StartResidentDelegate, CommandOutcome::ResidentDelegateStarted(observation)) => {
                serde_json::to_value(observation)
                    .map(CallToolResult::structured)
                    .map_err(|_| {
                        McpError::internal_error(
                            "Pale resident start result could not be encoded",
                            None,
                        )
                    })
            }
            (Self::AdvanceResidentInvoke, CommandOutcome::ResidentInvokeAdvanced(outcome)) => {
                serde_json::to_value(outcome)
                    .map(CallToolResult::structured)
                    .map_err(|_| {
                        McpError::internal_error("Pale Invoke result could not be encoded", None)
                    })
            }
            (Self::AdvanceResidentConsult, CommandOutcome::ResidentConsultAdvanced(outcome)) => {
                serde_json::to_value(outcome)
                    .map(CallToolResult::structured)
                    .map_err(|_| {
                        McpError::internal_error("Pale Consult result could not be encoded", None)
                    })
            }
            (Self::AdvanceResidentDelegate, CommandOutcome::ResidentDelegateAdvanced(outcome)) => {
                serde_json::to_value(outcome)
                    .map(CallToolResult::structured)
                    .map_err(|_| {
                        McpError::internal_error("Pale Delegate result could not be encoded", None)
                    })
            }
            (
                Self::CancelResidentOperation,
                CommandOutcome::ResidentOperationCancellation(outcome),
            ) => serde_json::to_value(outcome)
                .map(CallToolResult::structured)
                .map_err(|_| {
                    McpError::internal_error("Pale Cancel result could not be encoded", None)
                }),
            _ => Err(McpError::internal_error(
                "Pale command dispatcher returned an incompatible MCP outcome",
                None,
            )),
        }
    }
}

/// MCP object retaining the SDK's optional explanation without reclassification.
#[derive(JsonSchema, Serialize)]
#[schemars(deny_unknown_fields)]
struct EffectiveProfileExplanationOutput {
    /// Absence retains the SDK lookup result, not a fabricated stale diagnosis.
    explanation: Option<EffectiveProfileExplanation>,
}

/// Serialize the SDK-owned review result without reconstructing its payloads.
fn project_configuration_review(
    result: ConfigurationImprovementReviewResult,
) -> Result<CallToolResult, McpError> {
    serde_json::to_value(result)
        .map(CallToolResult::structured)
        .map_err(|_| {
            McpError::internal_error("Pale configuration review could not be encoded", None)
        })
}

impl ToolRoute {
    /// Translate an MCP object into one exact typed dispatcher request.
    ///
    /// # Errors
    ///
    /// Returns [`McpError`] when the object violates the advertised fixed-tool
    /// shape or cannot be encoded for API-owned action-payload admission.
    pub(super) fn command(&self, arguments: JsonObject) -> Result<CommandRequest, McpError> {
        match self {
            Self::Facade(route) => (*route).command(arguments),
            Self::NativeAction { action_id } => serde_json::to_vec(&arguments)
                .map(|payload| {
                    CommandRequest::NativeActionPayload(NativeActionPayloadRequest::new(
                        action_id.clone(),
                        payload,
                    ))
                })
                .map_err(|_| {
                    McpError::internal_error(
                        "Pale native-action payload could not be encoded",
                        None,
                    )
                }),
        }
    }

    /// Project the matching dispatcher outcome into MCP structured content.
    ///
    /// # Errors
    ///
    /// Returns [`McpError`] when the outcome does not match this route or its
    /// API-owned canonical bytes cannot enter MCP structured content.
    pub(super) fn project(&self, outcome: CommandOutcome) -> Result<CallToolResult, McpError> {
        match (self, outcome) {
            (Self::Facade(route), outcome) => (*route).project(outcome),
            (Self::NativeAction { .. }, CommandOutcome::NativeAction(execution)) => {
                projection::structured_bytes(execution.canonical_bytes())
            }
            _ => Err(McpError::internal_error(
                "Pale command dispatcher returned an incompatible MCP outcome",
                None,
            )),
        }
    }
}

/// Bind one sealed native-action descriptor to its exact MCP payload schema.
///
/// # Errors
///
/// Returns an error when the API-owned payload schema is not a JSON object.
pub(super) fn native_action_registration(
    action: &NativeActionDescriptor,
) -> ProcessResult<ToolRegistration> {
    let input_schema = serde_json::from_slice::<JsonObject>(
        action.canonical_payload_schema_bytes(),
    )
    .map_err(|error| {
        stage(
            "decode native-action MCP schema",
            format!("`{}`: {error}", action.action_id()),
        )
    })?;
    let description = action.description().to_owned();
    let title = native_action_title(action.mcp_tool_name());
    Ok(ToolRegistration {
        route: ToolRoute::NativeAction {
            action_id: action.action_id().to_owned(),
        },
        descriptor: Tool::new(
            action.mcp_tool_name().to_owned(),
            description.clone(),
            input_schema,
        )
        .with_title(title)
        .with_annotations(effect_free_annotations())
        .with_output_schema::<NativeActionExecution>(),
    })
}

/// Turn one admitted lower-snake-case tool name into a compact display title.
fn native_action_title(tool_name: &str) -> String {
    let mut title = tool_name.replace('_', " ");
    if let Some(first) = title.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    title
}

/// Maximum encoded path returned for one rejected tool argument.
///
/// Current generated paths are much shorter. The ceiling prevents a malformed
/// input from turning transport diagnostics into an unbounded reflection path.
const MAX_ARGUMENT_ERROR_PATH_BYTES: usize = 256;

/// Maximum decoder message prefix inspected for a stable error class.
const MAX_DECODER_ERROR_PREFIX_BYTES: usize = 128;

/// Bounded formatter used only to classify an exact-pinned decoder error.
struct DecoderErrorPrefix(String);

impl std::fmt::Write for DecoderErrorPrefix {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let remaining = MAX_DECODER_ERROR_PREFIX_BYTES.saturating_sub(self.0.len());
        let mut end = remaining.min(value.len());
        while !value.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        self.0.push_str(&value[..end]);
        Ok(())
    }
}

fn decoder_error_prefix(error: &serde_json::Error) -> String {
    let mut prefix = DecoderErrorPrefix(String::new());
    let _ = write!(&mut prefix, "{error}");
    prefix.0
}

/// Stable private classification for one Serde decoder rejection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArgumentDecodeClass {
    MissingField,
    UnknownField,
    DuplicateField,
    UnknownVariant,
    InvalidType,
    InvalidValue,
    InvalidLength,
    Data,
    Syntax,
    EndOfInput,
    Io,
}

impl ArgumentDecodeClass {
    /// Classify the exact-pinned decoder error without returning its message.
    fn from_error(error: &serde_json::Error, message: &str) -> Self {
        if message.starts_with("missing field `") {
            Self::MissingField
        } else if message.starts_with("unknown field `") {
            Self::UnknownField
        } else if message.starts_with("duplicate field `") {
            Self::DuplicateField
        } else if message.starts_with("unknown variant `") {
            Self::UnknownVariant
        } else if message.starts_with("invalid type:") {
            Self::InvalidType
        } else if message.starts_with("invalid value:") {
            Self::InvalidValue
        } else if message.starts_with("invalid length ") {
            Self::InvalidLength
        } else {
            match error.classify() {
                serde_json::error::Category::Data => Self::Data,
                serde_json::error::Category::Syntax => Self::Syntax,
                serde_json::error::Category::Eof => Self::EndOfInput,
                serde_json::error::Category::Io => Self::Io,
            }
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::MissingField => "missing_field",
            Self::UnknownField => "unknown_field",
            Self::DuplicateField => "duplicate_field",
            Self::UnknownVariant => "unknown_variant",
            Self::InvalidType => "invalid_type",
            Self::InvalidValue => "invalid_value",
            Self::InvalidLength => "invalid_length",
            Self::Data => "data",
            Self::Syntax => "syntax",
            Self::EndOfInput => "end_of_input",
            Self::Io => "io",
        }
    }
}

/// One bounded field path, with truncation reported instead of hidden.
struct ArgumentDecodePath {
    path: String,
    truncated: bool,
}

impl ArgumentDecodePath {
    fn from_error(
        error: &serde_path_to_error::Error<serde_json::Error>,
        class: ArgumentDecodeClass,
        decoder_prefix: &str,
    ) -> Self {
        let mut path = "$".to_owned();
        let mut truncated = false;
        let segment_count = error.path().iter().len();
        let visible_segments = if class == ArgumentDecodeClass::UnknownField {
            // The final segment is caller-controlled and can itself contain a
            // secret. Its parent still identifies the rejected object.
            segment_count.saturating_sub(1)
        } else {
            segment_count
        };
        for segment in error.path().iter().take(visible_segments) {
            let appended = match segment {
                serde_path_to_error::Segment::Seq { index } => {
                    push_path_fragment(&mut path, &format!("[{index}]"))
                }
                serde_path_to_error::Segment::Map { key } => {
                    push_path_fragment(&mut path, ".") && push_path_fragment(&mut path, key)
                }
                serde_path_to_error::Segment::Enum { .. }
                | serde_path_to_error::Segment::Unknown => {
                    // Enum spellings and unknown non-string keys are supplied
                    // by the caller, not the advertised schema.
                    push_path_fragment(&mut path, ".<redacted>")
                }
            };
            if !appended {
                truncated = true;
                break;
            }
        }

        // Missing fields have no input value for the path tracker to visit.
        // Serde's field name is schema-owned and safe to add; no rejected
        // caller value or arbitrary unknown-field spelling is reflected.
        if class == ArgumentDecodeClass::MissingField {
            if let Some(field) = missing_field_name(decoder_prefix) {
                truncated |=
                    !(push_path_fragment(&mut path, ".") && push_path_fragment(&mut path, field));
            }
        }

        if truncated {
            Self {
                path: "$".to_owned(),
                truncated: true,
            }
        } else {
            Self {
                path,
                truncated: false,
            }
        }
    }
}

fn push_path_fragment(path: &mut String, fragment: &str) -> bool {
    if path.len().saturating_add(fragment.len()) > MAX_ARGUMENT_ERROR_PATH_BYTES {
        return false;
    }
    path.push_str(fragment);
    true
}

/// Extract only a schema-owned missing-field name from Serde's pinned form.
fn missing_field_name(message: &str) -> Option<&str> {
    message
        .strip_prefix("missing field `")
        .and_then(|message| message.strip_suffix('`'))
}

/// Enforce the same closed shape advertised by the derived MCP descriptor.
fn parse<T: DeserializeOwned>(arguments: JsonObject) -> Result<T, McpError> {
    let value = serde_json::Value::Object(arguments);
    serde_path_to_error::deserialize(value.into_deserializer()).map_err(|error| {
        let decoder_prefix = decoder_error_prefix(error.inner());
        let class = ArgumentDecodeClass::from_error(error.inner(), &decoder_prefix);
        let location = ArgumentDecodePath::from_error(&error, class, &decoder_prefix);
        McpError::invalid_params(
            "tool arguments do not satisfy the advertised Pale input shape",
            Some(serde_json::json!({
                "code": "pale_mcp_invalid_tool_arguments",
                "decoder_class": class.code(),
                "path": location.path,
                "path_truncated": location.truncated,
            })),
        )
    })
}

/// Derive one exact input schema and attach route-specific effect annotations.
fn registration<T: JsonSchema + 'static>(
    route: FacadeTool,
    name: &'static str,
    description: &'static str,
) -> ProcessResult<ToolRegistration> {
    let schema = schema_for_input::<T>()
        .map_err(|error| stage("derive MCP input schema", format!("`{name}`: {error}")))?;
    let schema = qualify_signed_request_description(route, name, schema)?;
    let descriptor = Tool::new(name, description, schema)
        .with_title(route.title())
        .with_annotations(route.annotations());
    Ok(ToolRegistration {
        route: ToolRoute::Facade(route),
        descriptor: route.with_supported_output_schema(descriptor),
    })
}

/// Give a reused parsing shape the exact request meaning of its tool route.
fn qualify_signed_request_description(
    route: FacadeTool,
    name: &'static str,
    schema: Arc<JsonObject>,
) -> ProcessResult<Arc<JsonObject>> {
    let Some(description) = (match route {
        FacadeTool::AdvanceResidentInvoke => {
            Some("Exact pre-signed Invoke continuation request as canonical JSON text.")
        }
        FacadeTool::AdvanceResidentConsult => {
            Some("Exact pre-signed Consult continuation request as canonical JSON text.")
        }
        FacadeTool::AdvanceResidentDelegate => {
            Some("Exact pre-signed Delegate continuation request as canonical JSON text.")
        }
        FacadeTool::CancelResidentOperation => {
            Some("Exact pre-signed Cancel request as canonical JSON text.")
        }
        _ => None,
    }) else {
        return Ok(schema);
    };

    let mut schema = schema.as_ref().clone();
    let property = schema
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|properties| properties.get_mut("signedCommonRequest"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| {
            stage(
                "qualify MCP input schema",
                format!("`{name}` has no signedCommonRequest property"),
            )
        })?;
    property.insert(
        "description".to_owned(),
        serde_json::Value::String(description.to_owned()),
    );
    Ok(Arc::new(schema))
}

/// Construct the common MCP hints for every deterministic effect-free tool.
fn effect_free_annotations() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(true)
        .destructive(false)
        .idempotent(true)
        .open_world(false)
}

/// A durable start mutates resident custody but does not dispatch externally.
fn resident_start_annotations() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(false)
        .idempotent(true)
        .open_world(false)
}

/// One bounded advance may progress a later phase when the caller repeats it.
fn resident_advance_annotations() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(true)
        .idempotent(false)
        .open_world(true)
}

/// Cancellation is replay-safe but may cross an admitted native or remote boundary.
fn resident_cancellation_annotations() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(true)
        .idempotent(true)
        .open_world(true)
}
