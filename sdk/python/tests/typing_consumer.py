"""Strict public-package consumer; the private native stub is not imported."""

from typing import Optional, Union

from pypale import (
    ObservationSnapshot, ObservationSubject, ObservationProfileResultKind,
    ObservationProfileError, ObservationProfilePage, ObservationProfileContinuation,
    ObservationCancellation, ObservationCursor, ObservationFact, ObservationFilter,
    ObservationLens, ObservationLensDispositionKind, ObservationLensLimits,
    ObservationLensNode,
    AbsentWorkspaceSubjectValues,
    PriorOnlyWorkspaceSubjectValues,
    CurrentOnlyWorkspaceSubjectValues,
    SharedWorkspaceSubjectValues,
    WorkspaceComparison,
    WorkspaceComparisonSubjectCursor,
    WorkspaceComparisonSubjectPage,
    WorkspaceComparisonSummary,
    PortableWorkspaceDependencyCursor,
    PortableWorkspaceDependencyPage,
    WorkspaceCausalStep,
    WorkspaceSubject,
    WorkspaceSubjectComparison,
    WorkspaceFact,
    WorkspaceImpactPage,
    WorkspaceValueComparison,
    AuthoringCorrection,
    AuthoringCorrectionKind,
    AuthoringInstancePathSegment,
    AuthoringSourceLocation,
    AuthoringIdentity,
    AuthoringOccurrencePage,
    AuthoringOccurrencePageCursor,
    EffectiveAuthoringPage,
    EffectiveAuthoringContinuation,
    BuiltWorkspaceModule,
    ConfigurationImprovementCandidateRequest,
    ConfigurationImprovementDiagnosis,
    ConfigurationImprovementEvaluation,
    ConfigurationImprovementExplanation,
    ConfigurationImprovementInspection,
    ConfigurationImprovementPreview,
    ConfigurationImprovementProposal,
    ConfigurationImprovementProposalRequest,
    ConfigurationImprovementTerminal,
    Diagnostic,
    EffectiveProfileCompilation,
    EffectiveProfileEntry,
    EffectiveProfileApplicationParticipant,
    EffectiveProfileExecutionPathDetail,
    EffectiveProfileExtensionResponsibility,
    EffectiveProfileRepositoryDetail,
    EffectiveProfileResponsibilityDetail,
    EffectiveProfileSection,
    EffectiveProfileExplanation,
    EffectiveProfileDeclaredSelectionEvidence,
    EffectiveProfileResponsibilityEvidence,
    EffectiveProfileEffectiveSelectionEvidence,
    EffectiveProfileExecutionPathEvidence,
    EffectiveProfileOperationClassificationEvidence,
    EffectiveProfileCapabilityClassificationEvidence,
    EffectiveProfileFindingClassificationEvidence,
    EffectiveProfileMachineRealizationEvidence,
    EffectiveProfileConfigurationEvidence,
    EffectiveProfileBoundProviderDisposition,
    EffectiveProfileMissingBindingProviderDisposition,
    EffectiveProfileExcludedByNarrowingProviderDisposition,
    EffectiveProfileEffectMismatchProviderDisposition,
    EffectiveProfileRecipeContractMismatchProviderDisposition,
    EffectiveProfileNativeRealizationUnknownFinding,
    EffectiveProfileOperationUnboundFinding,
    EffectiveProfileRequirementFinding,
    EffectiveProfileOriginReceiptReason,
    EffectiveProfileEntryContractReason,
    EffectiveProfileResponsibilityContractReason,
    EffectiveProfileProviderBindingReason,
    EffectiveProfileDelegationEdgeReason,
    EffectiveProfileTerminationContractReason,
    EffectiveProfileSettlementContractReason,
    EffectiveProfileContinuityContractReason,
    SatisfiedRequirementDisposition,
    ProviderBindingAbsentRequirementDisposition,
    NarrowedRequirementDisposition,
    BlockedRequirementDisposition,
    NarrowingReason, PolicyBlock, OperationExposure,
    EffectiveProfileNativeRealization, EffectiveProfileProviderKind,
    EffectiveProfilePackageSelection,
    EffectiveProfileNormativePackSelection,
    EffectiveProfileSemanticResourceSelection,
    EffectiveProfileRunPolicySelection,
    EffectiveProfileSessionSelection,
    EffectiveProfileLifecycleSelection,
    EffectiveProfileAuthorityRootSelection,
    EffectiveProfileDisclosureCeilingSelection,
    EffectiveProfileDeclaredExactSelector,
    EffectiveProfileDeclaredAllFromPackageSelector,
    EffectiveProfileDeclaredResourceSelection,
    EffectiveProfileDeclaredModeDefaultSelection,
    EffectiveProfileDeclaredSelectionDetail,
    LocalConfigurationImprovementRequest,
    Pale,
    PaleDiagnosticError,
    PortableWorkspaceCompilation,
    PreparedConfigurationImprovementPreview,
    WorkspaceModuleBuilder,
    WorkspaceModulePreimage,
    WorkspaceImportSubjectValue,
    WholeWorkspaceSubjectValue,
    build_workspace_module,
)


def discover_subjects(owner: WorkspaceComparison, cursor: Optional[WorkspaceComparisonSubjectCursor]) -> WorkspaceComparisonSubjectPage:
    page = owner.subjects(1, cursor)
    summary: WorkspaceComparisonSummary = page.comparison()
    subjects: list[WorkspaceSubject] = page.subjects()
    inspected: int = page.inspected
    comparisons: int = page.key_comparisons
    next_cursor: Optional[WorkspaceComparisonSubjectCursor] = page.continuation
    exact_owner: bool = page.belongs_to(owner)
    complete: bool = page.is_complete
    encoded: bytes = summary.canonical_bytes
    assert inspected >= comparisons and exact_owner and subjects
    assert (next_cursor is None) == complete and encoded
    return page


def proof_inputs(owner: PortableWorkspaceCompilation, subject: WorkspaceSubject,
                 cursor: Optional[PortableWorkspaceDependencyCursor]) -> PortableWorkspaceDependencyPage:
    page = owner.direct_dependencies(subject, 1, cursor)
    root: Optional[WorkspaceCausalStep] = page.root()
    inputs: list[WorkspaceCausalStep] = page.dependencies()
    copied: WorkspaceSubject = page.subject()
    after: Optional[PortableWorkspaceDependencyCursor] = page.continuation
    encoded: bytes = page.canonical_bytes
    if root is not None:
        fact: WorkspaceFact = root.fact()
        assert fact.kind is not None and root.node_digest
    assert copied.kind is not None and encoded
    assert (after is None) == page.is_complete
    for step in inputs:
        assert step.subject().kind is not None and step.provenance().kind is not None
    return page


def compare_subject(owner: WorkspaceComparison, subject: WorkspaceSubject) -> str:
    """Narrow every presence variant from generated public-package types."""
    result: WorkspaceSubjectComparison = owner.compare_subject(subject)
    encoded: bytes = result.canonical_bytes
    returned_subject: WorkspaceSubject = result.subject()
    values: Union[AbsentWorkspaceSubjectValues, PriorOnlyWorkspaceSubjectValues,
                  CurrentOnlyWorkspaceSubjectValues, SharedWorkspaceSubjectValues] = result.values
    if isinstance(values, AbsentWorkspaceSubjectValues):
        return result.receipt_digest + str(returned_subject.kind) + str(len(encoded))
    if isinstance(values, PriorOnlyWorkspaceSubjectValues):
        predecessor: WorkspaceFact = values.prior
        return str(predecessor.kind)
    if isinstance(values, CurrentOnlyWorkspaceSubjectValues):
        current: WorkspaceFact = values.current
        return str(current.kind)
    comparison: WorkspaceValueComparison = values.comparison
    return str(values.prior.kind) + str(values.current.kind) + str(comparison)


def impact_page(owner: WorkspaceComparison) -> bytes:
    """Preserve the installed package's exact immutable impact-page payload."""
    page: WorkspaceImpactPage = owner.impacts(0, 1)
    encoded: bytes = page.canonical_bytes
    next_offset: Optional[int] = page.next_offset
    assert page.entries and (next_offset is None or next_offset > 0)
    return encoded


SelectionValue = Union[
    EffectiveProfilePackageSelection, EffectiveProfileNormativePackSelection,
    EffectiveProfileSemanticResourceSelection, EffectiveProfileRunPolicySelection,
    EffectiveProfileSessionSelection, EffectiveProfileLifecycleSelection,
    EffectiveProfileAuthorityRootSelection, EffectiveProfileDisclosureCeilingSelection,
]


def declared_selector_text(value: Union[EffectiveProfileDeclaredExactSelector, EffectiveProfileDeclaredAllFromPackageSelector]) -> str:
    if isinstance(value, EffectiveProfileDeclaredExactSelector):
        return value.resource_id
    return value.package_id


def declared_selection_text(value: Union[EffectiveProfileDeclaredResourceSelection, EffectiveProfileDeclaredModeDefaultSelection]) -> str:
    if isinstance(value, EffectiveProfileDeclaredResourceSelection):
        return str(value.resource_kind) + declared_selector_text(value.selector)
    return str(value.axis) + value.resource_id


def declared_explanation_text(value: EffectiveProfileExplanation) -> str:
    evidence = value.evidence
    return declared_selection_text(evidence.selection) if isinstance(evidence, EffectiveProfileDeclaredSelectionEvidence) else value.handle


def selection_text(value: SelectionValue) -> str:
    """Each concrete selection exposes required fields after narrowing."""
    if isinstance(value, EffectiveProfilePackageSelection):
        return value.package_id + value.version + value.content_digest
    if isinstance(value, EffectiveProfileNormativePackSelection):
        return value.pack_id + value.attestor_id + value.version + value.content_digest
    if isinstance(value, EffectiveProfileSemanticResourceSelection):
        return value.repository_id + str(value.resource_kind) + value.resource_id
    if isinstance(value, EffectiveProfileRunPolicySelection):
        return value.run_policy_id
    if isinstance(value, EffectiveProfileSessionSelection):
        count: int = value.activation_root_count
        return value.execution_workspace_id + str(count + value.additional_root_count + value.activation_path_count)
    if isinstance(value, EffectiveProfileLifecycleSelection):
        return value.lifecycle_id + value.event_id + value.action_id
    if isinstance(value, EffectiveProfileAuthorityRootSelection):
        return value.authority_root_id
    return value.authority_coordinate


def observation_profile_rows(snapshot: ObservationSnapshot, workspace: PortableWorkspaceCompilation,
                             profile: EffectiveProfileCompilation, subject: ObservationSubject) -> list[EffectiveProfileEntry]:
    result = snapshot.inspect_profile(profile, workspace, subject, 1, 1)
    if result.kind == ObservationProfileResultKind.Refused:
        error: Optional[ObservationProfileError] = result.error
        assert error is not None
        return []
    page: Optional[ObservationProfilePage] = result.page
    assert page is not None
    continuation: Optional[ObservationProfileContinuation] = page.continuation
    if continuation is not None:
        snapshot.inspect_profile(profile, workspace, subject, 1, 1, continuation)
    return page.entries


def observation_lens(snapshot: ObservationSnapshot) -> ObservationLens:
    """Keep exact lens facts, source coordinates and both omission counts typed."""
    result = snapshot.visual_lens(
        ObservationFilter(),
        ObservationLensLimits(4, 4, 2**32 - 1, 4, 256),
        False,
        ObservationCancellation.Active, None,
    )
    assert result.kind == ObservationLensDispositionKind.Ready
    lens: Optional[ObservationLens] = result.lens
    assert lens is not None
    cursor: ObservationCursor = lens.cursor
    nodes: list[ObservationLensNode] = lens.nodes
    facts: list[ObservationFact] = [fact for node in nodes for fact in node.facts]
    bounded_work: tuple[int, int, int, int, int] = (
        lens.inspected_subjects, lens.copied_string_bytes,
        lens.causal_path_lookups, lens.causal_steps, lens.snapshot_omitted_count,
    )
    assert cursor.revision is not None and facts
    assert all(value >= 0 for value in bounded_work)
    return lens


def profile_rows(profile: EffectiveProfileCompilation) -> list[EffectiveProfileEntry]:
    """Consume typed rows without parsing canonical page bytes."""
    return profile.inspect(EffectiveProfileSection.IntendedUseCase, 0, 64).entries


def authoring_rows(profile: EffectiveProfileCompilation, identity: AuthoringIdentity) -> list[EffectiveProfileEntry]:
    return profile.inspect_effective_authoring(identity).entries


def authoring_pages(workspace: PortableWorkspaceCompilation, profile: EffectiveProfileCompilation,
                    identity: AuthoringIdentity) -> tuple[AuthoringOccurrencePage, EffectiveAuthoringPage]:
    occurrences: AuthoringOccurrencePage = workspace.authoring_occurrences(identity, 1, 1)
    cursor: Union[AuthoringOccurrencePageCursor, None] = occurrences.continuation
    if cursor is not None:
        occurrences = workspace.authoring_occurrences(identity, 1, 1, cursor)
    native_occurrences: AuthoringOccurrencePage = profile.authoring_occurrences(identity, 1, 1)
    inspected: int = native_occurrences.inspected
    copied_string_bytes: int = native_occurrences.copied_string_bytes
    complete: bool = native_occurrences.is_complete
    for occurrence in native_occurrences.occurrences:
        location: AuthoringSourceLocation = occurrence.location
        assert location.source_label and inspected >= 0
    rows: EffectiveAuthoringPage = profile.inspect_effective_authoring_page(identity, 1)
    continuation: Union[EffectiveAuthoringContinuation, None] = rows.continuation
    if continuation is not None:
        rows = profile.inspect_effective_authoring_page(identity, 1, continuation)
    assert isinstance(complete, bool) and isinstance(rows.coordinates.instance_digest, str)
    assert inspected >= 0 and copied_string_bytes >= 0 and complete in (True, False)
    return occurrences, rows


def row_text(row: EffectiveProfileEntry) -> str:
    """Narrow once, then read required fields from the retained payload."""
    detail = row.detail
    if isinstance(detail, EffectiveProfileDeclaredSelectionDetail):
        return declared_selection_text(detail.selection)
    if isinstance(detail, EffectiveProfileRepositoryDetail):
        identity: str = detail.repository_id
        order: int = detail.materialization_order
        return identity + str(order)
    if isinstance(detail, EffectiveProfileExecutionPathDetail):
        binding = detail.terminal_provider_binding
        return detail.selection_id + (binding.provider_binding_id if binding is not None else "")
    if isinstance(detail, EffectiveProfileResponsibilityDetail):
        result: str = detail.contract_id
        participant = detail.participant
        if isinstance(participant, EffectiveProfileApplicationParticipant):
            result += participant.application_id
        for responsibility in detail.responsibilities:
            if isinstance(responsibility, EffectiveProfileExtensionResponsibility):
                result += responsibility.schema_id + responsibility.schema_sha256
        subject = detail.subject
        if isinstance(subject, WorkspaceImportSubjectValue):
            result += subject.upstream_release_id
        elif isinstance(subject, WholeWorkspaceSubjectValue):
            result += subject.workspace_id
        return result
    return row.explanation_handle


RequirementValue = Union[SatisfiedRequirementDisposition, ProviderBindingAbsentRequirementDisposition, NarrowedRequirementDisposition, BlockedRequirementDisposition]
ProviderValue = Union[EffectiveProfileBoundProviderDisposition, EffectiveProfileMissingBindingProviderDisposition, EffectiveProfileExcludedByNarrowingProviderDisposition, EffectiveProfileEffectMismatchProviderDisposition, EffectiveProfileRecipeContractMismatchProviderDisposition]
FindingValue = Union[EffectiveProfileNativeRealizationUnknownFinding, EffectiveProfileOperationUnboundFinding, EffectiveProfileRequirementFinding]
PathReasonValue = Union[EffectiveProfileOriginReceiptReason, EffectiveProfileEntryContractReason, EffectiveProfileResponsibilityContractReason, EffectiveProfileProviderBindingReason, EffectiveProfileDelegationEdgeReason, EffectiveProfileTerminationContractReason, EffectiveProfileSettlementContractReason, EffectiveProfileContinuityContractReason]


def requirement_text(value: RequirementValue) -> str:
    if isinstance(value, NarrowedRequirementDisposition):
        narrowed: NarrowingReason = value.reason
        return str(narrowed)
    if isinstance(value, BlockedRequirementDisposition):
        blocked: PolicyBlock = value.reason
        return str(blocked)
    return type(value).__name__


def provider_text(value: ProviderValue) -> str:
    if isinstance(value, EffectiveProfileBoundProviderDisposition):
        provider: EffectiveProfileProviderKind = value.provider
        return str(provider)
    return type(value).__name__


def finding_text(value: FindingValue) -> str:
    if isinstance(value, EffectiveProfileNativeRealizationUnknownFinding):
        return value.machine_id
    if isinstance(value, EffectiveProfileOperationUnboundFinding):
        return value.repository_id + value.operation_id
    return value.repository_id + value.operation_id + value.requirement_id + value.capability_id + requirement_text(value.disposition)


def reason_text(value: PathReasonValue) -> str:
    if isinstance(value, EffectiveProfileProviderBindingReason):
        return value.binding_id
    if isinstance(value, EffectiveProfileDelegationEdgeReason):
        return value.edge_id
    return value.contract_id


def explanation_payload(value: EffectiveProfileExplanation) -> str:
    """Read every required evidence payload without parsing canonical bytes."""
    evidence = value.evidence
    if isinstance(evidence, EffectiveProfileDeclaredSelectionEvidence):
        return declared_selection_text(evidence.selection)
    if isinstance(evidence, EffectiveProfileResponsibilityEvidence):
        return evidence.contract_id + evidence.execution_path_id + ''.join(map(reason_text, evidence.reason_steps))
    if isinstance(evidence, EffectiveProfileEffectiveSelectionEvidence):
        return selection_text(evidence.selection)
    if isinstance(evidence, EffectiveProfileExecutionPathEvidence):
        return evidence.selection_id + ''.join(map(reason_text, evidence.reason_steps))
    if isinstance(evidence, EffectiveProfileOperationClassificationEvidence):
        exposure: OperationExposure = evidence.exposure
        native: EffectiveProfileNativeRealization = evidence.native_realization
        return str(exposure) + str(native)
    if isinstance(evidence, EffectiveProfileCapabilityClassificationEvidence):
        capability_native: EffectiveProfileNativeRealization = evidence.native_realization
        return requirement_text(evidence.requirement_disposition) + provider_text(evidence.provider_disposition) + str(capability_native)
    if isinstance(evidence, EffectiveProfileFindingClassificationEvidence):
        return finding_text(evidence.finding)
    if isinstance(evidence, EffectiveProfileMachineRealizationEvidence):
        realization: EffectiveProfileNativeRealization = evidence.realization
        return str(realization)
    configuration: EffectiveProfileConfigurationEvidence = evidence
    return value.handle + type(configuration).__name__


def local_preview(client: Pale, baseline: str, candidate: str) -> ConfigurationImprovementPreview:
    """The public request returns the same typed preview, not an untyped source map."""
    return client.preview_local_configuration_improvement(
        LocalConfigurationImprovementRequest(
            baseline_manifest=baseline, candidate_manifest=candidate,
            project_id="demo", execution_workspace_id="default", machine_id="machine",
        )
    )


def diagnostic_view(error: PaleDiagnosticError) -> tuple[Diagnostic, bytes]:
    """Retain both canonical projections across Python's exception boundary."""
    return error.diagnostic, error.diagnostic_bytes


def authoring_failure(error: PaleDiagnosticError) -> tuple[list[AuthoringCorrection], Union[AuthoringSourceLocation, None]]:
    """Retain typed compiler corrections and an optional exact source location."""
    for correction in error.authoring_corrections:
        text: str = correction.source_label + correction.expected_source + correction.replacement_source + correction.diagnostic_identity
        identity: AuthoringIdentity = correction.proof_subject
        coordinates: list[AuthoringInstancePathSegment] = correction.path
        kind: AuthoringCorrectionKind = correction.kind
        assert text and identity.value and len(coordinates) >= 0
        assert kind == AuthoringCorrectionKind.NarrowImportVisibility
    return error.authoring_corrections, error.authoring_location


def build(candidate: WorkspaceModulePreimage, client: Pale) -> BuiltWorkspaceModule:
    """Both public builder exports retain their typed result."""
    first: BuiltWorkspaceModule = WorkspaceModuleBuilder(client).build(candidate)
    second: BuiltWorkspaceModule = build_workspace_module(candidate, client=client)
    assert first == second
    return second


def improve_direct(
    client: Pale,
    baseline: PortableWorkspaceCompilation,
    profile: EffectiveProfileCompilation,
    request: ConfigurationImprovementProposalRequest,
) -> ConfigurationImprovementTerminal:
    """Retain every phase type across the direct, effect-free route."""
    inspected: ConfigurationImprovementInspection = (
        client.inspect_configuration_improvement(baseline)
    )
    explained: ConfigurationImprovementExplanation = (
        client.explain_configuration_improvement(inspected, profile)
    )
    proposed: ConfigurationImprovementProposal = (
        client.propose_configuration_improvement(explained, request)
    )
    evaluated: ConfigurationImprovementEvaluation = (
        client.evaluate_configuration_improvement(proposed)
    )
    diagnosed: ConfigurationImprovementDiagnosis = (
        client.diagnose_configuration_improvement(evaluated)
    )
    previewed: ConfigurationImprovementPreview = (
        client.preview_configuration_improvement(diagnosed)
    )
    return previewed.accept()


def improve_prepared(
    client: Pale,
    baseline: PortableWorkspaceCompilation,
    candidate: PortableWorkspaceCompilation,
    profile: EffectiveProfileCompilation,
    request: ConfigurationImprovementCandidateRequest,
) -> ConfigurationImprovementTerminal:
    """Retain the borrowed preparation and exact-owner commit types."""
    inspected: ConfigurationImprovementInspection = (
        client.inspect_configuration_improvement(baseline)
    )
    explained: ConfigurationImprovementExplanation = (
        client.explain_configuration_improvement(inspected, profile)
    )
    prepared: PreparedConfigurationImprovementPreview = (
        client.prepare_configuration_improvement_preview(
            explained, candidate, request
        )
    )
    previewed: ConfigurationImprovementPreview = (
        client.commit_configuration_improvement_preview(explained, prepared)
    )
    return previewed.mark_stale()
