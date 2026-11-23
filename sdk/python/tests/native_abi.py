"""Execute one shared conformance vector through the installed Python ABI."""

import ctypes
import gc
import hashlib
import json
import os
import pathlib
import sys
import tempfile

fixture_root = pathlib.Path(sys.argv[1])
fixture = json.loads((fixture_root / "bundle-identity.json").read_text(encoding="utf-8"))
contract = json.loads((fixture_root / "contract-diagnostic.json").read_text(encoding="utf-8"))

import pypale as pale_python  # noqa: E402
from control_storage import exercise_control_storage  # noqa: E402
from configuration_improvement import (  # noqa: E402
    exercise_configuration_improvement, exercise_local_configuration_improvement,
)
from diagnostic_transport import exercise_diagnostic_transport  # noqa: E402
from authoring_errors import exercise_authoring_errors  # noqa: E402
from effective_profile_entries import exercise_effective_profile_entries  # noqa: E402
from authoring_pages import exercise_authoring_pages  # noqa: E402
from subject_comparison import exercise_subject_comparison  # noqa: E402
from observation_profile import exercise_observation_profile  # noqa: E402
from observation_conformance import exercise_observation_conformance  # noqa: E402


def _running_executable() -> pathlib.Path:
    """Return the executable path that Rust's current_exe() observes."""
    if sys.platform == "darwin":
        buffer = ctypes.create_string_buffer(4096)
        process = ctypes.CDLL(None, use_errno=True)
        process.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
        process.proc_pidpath.restype = ctypes.c_int
        length = process.proc_pidpath(os.getpid(), buffer, len(buffer))
        if length <= 0:
            raise OSError(ctypes.get_errno(), "proc_pidpath failed for the current process")
        return pathlib.Path(os.fsdecode(buffer.value)).resolve(strict=True)
    if sys.platform.startswith("linux"):
        return pathlib.Path("/proc/self/exe").resolve(strict=True)
    if sys.platform == "win32":
        buffer = ctypes.create_unicode_buffer(32_768)
        process = ctypes.WinDLL("kernel32", use_last_error=True)
        process.GetModuleFileNameW.argtypes = [
            ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_uint32,
        ]
        process.GetModuleFileNameW.restype = ctypes.c_uint32
        length = process.GetModuleFileNameW(None, buffer, len(buffer))
        if length == 0 or length >= len(buffer):
            raise OSError(
                ctypes.get_last_error(),
                "GetModuleFileNameW failed for the current process",
            )
        return pathlib.Path(buffer.value).resolve(strict=True)
    raise RuntimeError(f"native ABI executable discovery is unsupported on {sys.platform}")


for shared_type_name in (
    "OperationExposure", "NarrowingReason", "PolicyBlock",
):
    assert getattr(pale_python, shared_type_name).__name__ == shared_type_name
    assert not hasattr(pale_python, "Observation" + shared_type_name)

identity = pale_python.Pale().contract_bundle_identity()
assert identity.member_count == fixture["member_count"]
assert identity.schema_manifest_sha256 == fixture["schema_manifest_sha256"]
assert identity.schema_set_sha256 == fixture["schema_set_sha256"]

api = pale_python.Pale()
exercise_authoring_errors(api)
exercise_control_storage(api)
exercise_diagnostic_transport(api)
try:
    api.contract_schema("not-a-selected-schema")
except pale_python.PaleDiagnosticError as error:
    structured = json.loads(error.diagnostic_bytes)
    assert str(error) == structured["summary"]
    assert error.diagnostic.summary == structured["summary"]
    assert error.diagnostic.code == "contract_rejected"
    assert error.diagnostic.canonical_bytes == error.diagnostic_bytes
    assert structured["code"] == "contract_rejected"
else:
    raise AssertionError("unknown contract schema must be rejected")
schema = api.contract_schema(contract["schema"]["registry_id"])
assert schema.registry_id == contract["schema"]["registry_id"]
assert schema.schema_uri == contract["schema"]["schema_uri"]
assert schema.wire_schema_id == contract["schema"]["wire_schema_id"]
assert schema.schema_sha256 == contract["schema"]["schema_sha256"]
assert isinstance(schema.bytes, bytes)
assert len(schema.bytes) == contract["schema"]["byte_length"]

projection = pale_python.DiagnosticProjection(
    code=contract["projection"]["code"],
    stage=pale_python.DiagnosticStage.InputAdmission,
    disposition=pale_python.DiagnosticDisposition.InvalidInput,
    summary=contract["projection"]["summary"],
    retry=pale_python.DiagnosticRetry.AfterCorrection,
    effect=pale_python.DiagnosticEffectCertainty.NotApplicable,
)
projected_diagnostic = api.project_diagnostic(projection)
assert isinstance(projected_diagnostic, bytes)
diagnostic = projected_diagnostic.decode()
assert diagnostic == contract["canonical_diagnostic"]
outcome_unknown = pale_python.DiagnosticProjection(
    code=contract["outcome_unknown"]["code"],
    stage=pale_python.DiagnosticStage.Materialization,
    disposition=pale_python.DiagnosticDisposition.EffectOutcomeUnknown,
    summary=contract["outcome_unknown"]["summary"],
    retry=pale_python.DiagnosticRetry.ReconcileFirst,
    effect=pale_python.DiagnosticEffectCertainty.OutcomeUnknown,
)
assert api.project_diagnostic(outcome_unknown).decode() == contract["outcome_unknown"][
    "canonical_diagnostic"
]
recovery_projection = pale_python.DiagnosticProjection(
    code="resident_recovery_required",
    stage=pale_python.DiagnosticStage.ResidentRecovery,
    disposition=pale_python.DiagnosticDisposition.Blocked,
    summary="resident recovery evidence must be reconciled",
    retry=pale_python.DiagnosticRetry.ReconcileFirst,
    effect=pale_python.DiagnosticEffectCertainty.NotApplicable,
)
recovery_diagnostic = json.loads(api.project_diagnostic(recovery_projection))
assert recovery_diagnostic["stage"] == "resident_recovery"
assert recovery_diagnostic["retry"] == "reconcile_first"
assert recovery_diagnostic["effect"] == "not_applicable"
admitted_diagnostic = api.admit_diagnostic(contract["canonical_diagnostic"].encode())
assert isinstance(admitted_diagnostic, bytes)
assert admitted_diagnostic.decode() == diagnostic
validated = api.validate_contract_document(
    contract["schema"]["registry_id"], contract["canonical_diagnostic"].encode()
)
assert validated.schema_id == contract["schema"]["registry_id"]
assert validated.schema_sha256 == contract["schema"]["schema_sha256"]
assert isinstance(validated.canonical_bytes, bytes)
assert validated.canonical_bytes.decode() == diagnostic

portable = json.loads((fixture_root / "portable-journey.json").read_text(encoding="utf-8"))
documents = [(label, (fixture_root / "portable" / label).read_bytes()) for label in portable["document_labels"]]
workspace_limits = api.authoring_ingress_limits(
    ingress_class=pale_python.AuthoringIngressClass.Workspace
)
assert workspace_limits.max_documents == portable["workspace_authoring_limits"]["max_documents"]
assert workspace_limits.max_document_bytes == int(
    portable["workspace_authoring_limits"]["max_document_bytes"]
)
assert workspace_limits.max_total_bytes == int(
    portable["workspace_authoring_limits"]["max_total_bytes"]
)
api.validate_authoring_document(
    pale_python.StandaloneAuthoringRole.WorkspaceManifest,
    "pale-workspace.toml",
    (fixture_root / "portable/pale-workspace.toml").read_bytes(),
)
standalone_inspection = api.inspect_standalone_authoring(
    pale_python.StandaloneAuthoringRole.WorkspaceManifest,
    [pale_python.AuthoringPathSegment.field("workspace")],
)
assert standalone_inspection.state == pale_python.AuthoringInspectionState.Available
try:
    api.inspect_standalone_authoring(
        pale_python.StandaloneAuthoringRole.WorkspaceManifest,
        [
            pale_python.AuthoringPathSegment.field(f"over-bound-{index}")
            for index in range(65)
        ],
    )
except pale_python.PaleDiagnosticError as error:
    structured = json.loads(error.diagnostic_bytes)
    assert str(error) == structured["summary"]
    assert error.diagnostic.code == "authoring_path_too_deep"
    assert structured["stage"] == "input_admission"
    assert structured["disposition"] == "invalid_input"
    assert structured["retry"] == "after_correction"
    assert structured["effect"] == "not_applicable"
else:
    raise AssertionError("oversized authoring path must produce a canonical diagnostic")
prior = api.compile_portable_workspace(portable["root_manifest"], documents)
current = api.compile_portable_workspace(portable["root_manifest"], documents)
actual_compilation = {key: getattr(prior, key) for key in portable["compilation"]}
assert actual_compilation == portable["compilation"]
portable_inspection = prior.inspect_authoring(
    "pale-workspace.toml",
    [pale_python.AuthoringPathSegment.field("workspace")],
)
assert portable_inspection.state == pale_python.AuthoringInspectionState.Available
portable_navigation = prior.navigate_authoring(
    "pale-workspace.toml",
    [
        pale_python.AuthoringInstancePathSegment.field("workspace"),
        pale_python.AuthoringInstancePathSegment.field("workspace_id"),
    ],
)
assert portable_navigation.identity is not None
assert portable_navigation.identity.value == portable["compilation"]["workspace_id"]
comparison = api.compare_portable_workspaces(prior, current)
actual_comparison = {key: getattr(comparison, key) for key in portable["comparison"]}
assert actual_comparison == portable["comparison"]
roots = comparison.change_roots(0, 65535)
assert len(roots.entries) == portable["change_roots"]["entry_count"]
assert roots.next_offset == portable["change_roots"]["next_offset"]
impact_page = comparison.impacts(0, 1)
assert impact_page.next_offset == portable["first_impact"]["next_offset"]
impact_bytes = impact_page.canonical_bytes
assert isinstance(impact_bytes, bytes)
assert json.loads(impact_bytes) == {
    "entries": [{
        "subject": {
            "kind": portable["first_impact"]["kind"],
            "sourceId": portable["first_impact"]["source_id"],
        },
        "state": portable["first_impact"]["state"],
        "contributingRootCount": portable["first_impact"]["contributing_root_count"],
    }],
    "nextOffset": portable["first_impact"]["next_offset"],
}
repeated_impact_page = comparison.impacts(0, 1)
assert repeated_impact_page.canonical_bytes == impact_bytes
assert hashlib.sha256(repeated_impact_page.canonical_bytes).digest() == hashlib.sha256(
    impact_bytes
).digest()
try:
    impact_bytes[0] = 0
except TypeError:
    pass
else:
    raise AssertionError("canonical impact bytes must be immutable")
try:
    pale_python.WorkspaceImpactPage()
except TypeError:
    pass
else:
    raise AssertionError("impact pages must remain output-only")
try:
    comparison.impacts({"offset": 0}, 1)
except TypeError:
    pass
else:
    raise AssertionError("sparse impact coordinates must be rejected")
impact = impact_page.entries[0]
subject = impact.subject()
assert subject.kind == pale_python.WorkspaceSubjectKind.Source
assert subject.source_id == portable["first_impact"]["source_id"]
for field in ["workspace_id", "component_id", "module_id", "owner_module_id", "declaration_namespace_id", "declaration_id", "importer_module_id", "upstream_module_id", "upstream_release_id", "export_id", "source_namespace_id", "local_namespace_id", "extension_namespace_id", "resource_id"]:
    assert getattr(subject, field) is None
assert impact.state == pale_python.WorkspaceImpactState.Unaffected
assert impact.contributing_root_count == 0
explanation = comparison.explain(subject)
assert explanation.state == pale_python.WorkspaceExplanationState.Unaffected
assert explanation.frontier is None
assert explanation.root() is None
assert explanation.steps() == []
discovery = exercise_subject_comparison(api, portable, documents)
assert discovery == {"comparison_subjects": True, "direct_dependencies": True}

action = json.loads((fixture_root / "native-action-journey.json").read_text(encoding="utf-8"))
action_descriptors = api.native_action_catalog()
assert [descriptor.action_id for descriptor in action_descriptors] == action["descriptor_ids"]
assert all(isinstance(descriptor.contract_bytes, bytes) for descriptor in action_descriptors)
assert all(isinstance(descriptor.payload_schema_bytes, bytes) for descriptor in action_descriptors)
assert [[descriptor.action_id, descriptor.description, hashlib.sha256(descriptor.contract_bytes).hexdigest(), len(descriptor.contract_bytes), hashlib.sha256(descriptor.payload_schema_bytes).hexdigest(), len(descriptor.payload_schema_bytes)] for descriptor in action_descriptors] == action["descriptor_projections"]
tagged_execution = bytes(api.invoke_native_action(action["action_id"], action["tagged_input"].encode()))
payload_execution = bytes(api.invoke_native_action_payload(action["action_id"], action["payload_input"].encode()))
assert tagged_execution == payload_execution
assert len(payload_execution) == action["execution_length"]
assert hashlib.sha256(payload_execution).hexdigest() == action["execution_sha256"]

one_shot = json.loads((fixture_root / "one-shot-journey.json").read_text(encoding="utf-8"))
one_shot_root = fixture_root / "one-shot"
Document = pale_python.CompilationDocument
private_inputs = pale_python.OneShotPrivateInputs(
    capabilities=Document("capabilities.toml", (one_shot_root / "private/capabilities.toml").read_bytes()),
    machine_binding=Document("machines/machine.toml", (one_shot_root / "private/machine-binding.toml").read_bytes()),
    distribution_lock=Document("distribution-lock.toml", (one_shot_root / "private/distribution-lock.toml").read_bytes()),
    lifecycle_guidance=[],
)
package_documents = [Document(f"engine/{name}", (one_shot_root / "packages/engine" / name).read_bytes()) for name in ["package.toml", "role.md", "roles.toml"]]
workspace_documents = [Document(name, (one_shot_root / "workspace" / name).read_bytes()) for name in ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"]]
effective_workspace = api.compile_portable_workspace(
    "pale-workspace.toml",
    [(name, (one_shot_root / "workspace" / name).read_bytes()) for name in ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"]],
)
effective_request = pale_python.EffectiveProfileCompilationRequest(
    project_id=one_shot["selectors"]["project_id"],
    execution_workspace_id=one_shot["selectors"]["execution_workspace_id"],
    machine_id=one_shot["selectors"]["machine_id"],
    private_inputs=pale_python.OneShotPrivateInputs(
        capabilities=Document("capabilities.toml", (one_shot_root / "private/capabilities.toml").read_bytes()),
        machine_binding=Document("machines/machine.toml", (one_shot_root / "private/machine-binding.toml").read_bytes()),
        distribution_lock=Document("distribution-lock.toml", (one_shot_root / "private/distribution-lock.toml").read_bytes()),
        lifecycle_guidance=[],
    ),
    package_documents=[Document(f"engine/{name}", (one_shot_root / "packages/engine" / name).read_bytes()) for name in ["package.toml", "role.md", "roles.toml"]],
)
effective = api.compile_effective_profile(effective_workspace, effective_request)
for declared_role in (
    '{ selection = "exact", resource = { package_id = "example.engine", resource_id = "engine" } }',
    '{ selection = "all_from_package", package_id = "example.engine" }',
):
    # Same canonical private-selection route as the Rust SDK fixture; no corpus
    # mutation or source admission is implemented by this language test.
    declared_request = pale_python.EffectiveProfileCompilationRequest(
        project_id=one_shot["selectors"]["project_id"],
        execution_workspace_id=one_shot["selectors"]["execution_workspace_id"],
        machine_id=one_shot["selectors"]["machine_id"],
        private_inputs=pale_python.OneShotPrivateInputs(
            capabilities=Document("capabilities.toml",
                f"[semantic]\nroles = [{declared_role}]\n\n".encode()
                + (one_shot_root / "private/capabilities.toml").read_bytes()),
            machine_binding=Document("machines/machine.toml", (one_shot_root / "private/machine-binding.toml").read_bytes()),
            distribution_lock=Document("distribution-lock.toml", (one_shot_root / "private/distribution-lock.toml").read_bytes()),
            lifecycle_guidance=[],
        ),
        package_documents=package_documents,
    )
    declared_profile = api.compile_effective_profile(effective_workspace, declared_request)
    declared_page = declared_profile.inspect(pale_python.EffectiveProfileSection.IntendedUseCase, 0, 256)
    declared_rows = json.loads(declared_page.canonical_bytes)["entries"]
    declared_index = next(i for i, row in enumerate(declared_rows) if row["kind"] == "declared-selection")
    declared_row = declared_rows[declared_index]
    print(json.dumps({"declaredSelectionWitness": declared_row["selection"]}))
    selection = declared_page.entries[declared_index].detail.selection
    assert isinstance(selection, pale_python.EffectiveProfileDeclaredResourceSelection)
    from effective_profile_entries import _assert_entry, _assert_projection
    _assert_entry(declared_page.entries[declared_index], declared_row)
    declaration_explanation = declared_profile.explain(declared_row["explanationHandle"])
    assert declaration_explanation.handle == declared_row["explanationHandle"]
    assert isinstance(declaration_explanation.evidence, pale_python.EffectiveProfileDeclaredSelectionEvidence)
    _assert_projection(declaration_explanation.evidence.selection, declared_row["selection"], "DeclaredSelection")
    selector = selection.selector
    key = "resource_id" if declared_row["selection"]["selector"]["kind"] == "exact" else "package_id"
    coordinate = getattr(selector, key)
    try:
        setattr(selector, key, "changed copied declaration")
    except AttributeError:
        pass
    else:
        raise AssertionError("declared selector must remain frozen")
    del declared_profile, declared_page, selection, declaration_explanation
    gc.collect()
    assert getattr(selector, key) == coordinate

coordinates = effective.coordinates()
assert coordinates.project_id == one_shot["selectors"]["project_id"]
assert coordinates.execution_workspace_id == one_shot["selectors"]["execution_workspace_id"]
assert coordinates.profile_digest == effective.profile_digest
assert len(coordinates.selected_topology_digest) == 64
effective_page = effective.inspect(pale_python.EffectiveProfileSection.IntendedUseCase, 0, 1)
assert isinstance(effective_page.canonical_bytes, bytes)
effective_page_value = json.loads(effective_page.canonical_bytes)
assert effective_page_value["section"] == "intended-use-case"
assert effective_page.offset == effective_page_value["offset"]
assert effective_page.entry_count == len(effective_page_value["entries"])
assert effective_page.next_offset == effective_page_value.get("nextOffset")
effective_inspection = effective.inspect_authoring(
    "pale-workspace.toml",
    [pale_python.AuthoringPathSegment.field("workspace")],
)
assert effective_inspection.state == pale_python.AuthoringInspectionState.Available
run_policy_navigation = effective.navigate_authoring(
    "pale-workspace.toml",
    [
        pale_python.AuthoringInstancePathSegment.field("workspace"),
        pale_python.AuthoringInstancePathSegment.field("desired_topology"),
        pale_python.AuthoringInstancePathSegment.field("projects"),
        pale_python.AuthoringInstancePathSegment.index(0),
        pale_python.AuthoringInstancePathSegment.field("run_policy"),
    ],
)
assert run_policy_navigation.identity is not None
assert run_policy_navigation.identity.value == "standard"
effective_join = effective.inspect_effective_authoring(run_policy_navigation.identity)
exercise_authoring_pages(api, fixture_root, portable, documents, prior, current, effective)
assert effective_join.coordinates.profile_digest == effective.profile_digest
assert effective_join.coordinates.selected_topology_digest == coordinates.selected_topology_digest
assert effective_join.entries
assert not hasattr(effective_join, "explanation_handles")
assert not effective_join.truncated
retained_entry, profile_rows = exercise_effective_profile_entries(
    effective, effective_workspace, run_policy_navigation.identity,
)
machine_page = effective.inspect(
    pale_python.EffectiveProfileSection.MachineRealization, 0, 1
)
machine_page_value = json.loads(machine_page.canonical_bytes)
explanation_handle = machine_page_value["entries"][0]["explanationHandle"]
profile_explanation = effective.explain(explanation_handle)
assert profile_explanation is not None
assert profile_explanation.handle == explanation_handle
exercise_configuration_improvement(api, fixture_root)
observation_profile_capability = exercise_observation_profile(
    api, effective_workspace, effective,
    api.compile_effective_profile(effective_workspace, effective_request),
    api.compile_portable_workspace("pale-workspace.toml", [(name, (one_shot_root / "workspace" / name).read_bytes()) for name in ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"]]),
    api.compile_effective_profile(effective_workspace, declared_request),
    profile_rows,
)
observation = api.observe_effective_profile(effective)
assert observation.fact_count > 0
assert observation.omitted_fact_count == 0
assert observation.cursor.generation.profile_digest == effective.profile_digest
assert observation.cursor.generation.instance_digest == effective.instance_digest
assert observation.cursor.generation.composition_plan_digest == effective.composition_plan_digest
operation_filter = pale_python.ObservationFilter()
operation_filter.select_category(pale_python.ObservationCategory.OperationExposure)
operation_page = observation.page(
    operation_filter,
    pale_python.ObservationPageLimits(256, 64, 2**32 - 1),
    pale_python.ObservationCancellation.Active, None,
)
assert operation_page.kind == pale_python.ObservationPageResultKind.Ready
unfiltered_page = observation.page(
    pale_python.ObservationFilter(),
    pale_python.ObservationPageLimits(256, 64, 2**32 - 1),
    pale_python.ObservationCancellation.Active, None,
)
assert unfiltered_page.kind == pale_python.ObservationPageResultKind.Ready
assert unfiltered_page.page.continuation is None
assert [fact.operation_exposure for fact in operation_page.page.facts] == [
    fact.operation_exposure for fact in unfiltered_page.page.facts
    if fact.category == pale_python.ObservationCategory.OperationExposure
]
for fact in operation_page.page.facts:
    assert isinstance(fact.operation_exposure, pale_python.OperationExposure)
    assert fact.requirement_disposition is None
observation_filter = pale_python.ObservationFilter()
observation_page_result = observation.page(
    observation_filter,
    pale_python.ObservationPageLimits(256, 2, 2**32 - 1),
    pale_python.ObservationCancellation.Active, None,
)
assert observation_page_result.kind == pale_python.ObservationPageResultKind.Ready
assert len(observation_page_result.page.facts) == 2
assert observation_page_result.page.snapshot_omitted_count == 0
assert observation_page_result.page.continuation is not None
mismatched_observation_page = observation.page(
    observation_filter,
    pale_python.ObservationPageLimits(256, 3, 2**32 - 1),
    pale_python.ObservationCancellation.Active,
    observation_page_result.page.continuation,
)
assert mismatched_observation_page.kind == pale_python.ObservationPageResultKind.Resynchronize
assert (
    mismatched_observation_page.resynchronization.reason
    == pale_python.ObservationResynchronizationReason.ContinuationMismatch
)
observation_subject = next(
    fact.subject
    for fact in observation_page_result.page.facts
    if fact.subject is not None
)
causal = observation.causal_path(observation_subject)
assert causal.kind != pale_python.ObservationCausalResultKind.Resynchronize
caught_up = observation.delta_from(
    observation, 1, pale_python.ObservationCancellation.Active
)
assert caught_up.kind == pale_python.ObservationDeltaOutcomeKind.CaughtUp
cancelled_lens = observation.visual_lens(
    observation_filter,
    pale_python.ObservationLensLimits(4, 4, 2**32 - 1, 4, 256),
    True,
    pale_python.ObservationCancellation.Cancelled,
    None,
)
assert cancelled_lens.kind == pale_python.ObservationLensDispositionKind.Cancelled
ready_lens = observation.visual_lens(
    observation_filter,
    pale_python.ObservationLensLimits(4, 4, 2**32 - 1, 4, 256),
    False,
    pale_python.ObservationCancellation.Active,
    None,
)
assert ready_lens.kind == pale_python.ObservationLensDispositionKind.Ready
assert len(ready_lens.lens.nodes) <= 4
assert all(node.facts for node in ready_lens.lens.nodes)
assert ready_lens.lens.cursor is not None
assert ready_lens.lens.snapshot_omitted_count == 0
exercise_observation_conformance(pale_python, observation, fixture_root)
assert isinstance(profile_explanation.evidence, pale_python.EffectiveProfileMachineRealizationEvidence)
assert profile_explanation.evidence.realization == pale_python.EffectiveProfileNativeRealization.Unknown
assert not hasattr(profile_explanation, "evidence_kind")
assert effective.explain("0" * 64) is None
try:
    effective.explain("not-a-handle")
except ValueError:
    pass
else:
    raise AssertionError("malformed explanation handle must remain rejected")

evidence_classes = {
    "responsibility": pale_python.EffectiveProfileResponsibilityEvidence,
    "execution-path": pale_python.EffectiveProfileExecutionPathEvidence,
    "selection": pale_python.EffectiveProfileEffectiveSelectionEvidence,
    "repository": pale_python.EffectiveProfileConfigurationEvidence,
    "machine-realization": pale_python.EffectiveProfileMachineRealizationEvidence,
    "finding": pale_python.EffectiveProfileFindingClassificationEvidence,
}
seen_evidence = set()
for section in (
    pale_python.EffectiveProfileSection.IntendedUseCase,
    pale_python.EffectiveProfileSection.EffectiveConfiguration,
    pale_python.EffectiveProfileSection.MachineRealization,
    pale_python.EffectiveProfileSection.GapsAndExplanations,
):
    page = effective.inspect(section, 0, 256)
    assert page.next_offset is None
    for row in json.loads(page.canonical_bytes)["entries"]:
        result = effective.explain(row["explanationHandle"])
        assert result is not None
        assert result.handle == row["explanationHandle"]
        proof = result.evidence
        assert isinstance(proof, evidence_classes[row["kind"]])
        seen_evidence.add(row["kind"])
        if row["kind"] in ("responsibility", "execution-path"):
            if row["kind"] == "responsibility":
                assert proof.contract_id == row["contractId"]
                assert proof.execution_path_id == row["executionPathId"]
                retained_evidence = proof
            else:
                assert proof.selection_id == row["selectionId"]
            reasons = proof.reason_steps
            assert reasons is not None
            assert len(reasons) == len(row["reasonSteps"])
            for reason, expected_reason in zip(reasons, row["reasonSteps"]):
                _assert_projection(reason, expected_reason, "Reason")
            reasons.clear()
            assert len(proof.reason_steps) == len(row["reasonSteps"])
        elif row["kind"] == "selection":
            selected = proof.selection
            assert selected is not None
            expected = row["selection"]
            if expected["kind"] == "package":
                assert isinstance(selected, pale_python.EffectiveProfilePackageSelection)
                assert (selected.package_id, selected.version, selected.content_digest) == (
                    expected["packageId"], expected["version"], expected["contentDigest"],
                )
            elif expected["kind"] == "run-policy":
                assert isinstance(selected, pale_python.EffectiveProfileRunPolicySelection)
                assert selected.run_policy_id == expected["runPolicyId"]
                retained_selection = selected
                retained_policy = selected.run_policy_id
            elif expected["kind"] == "authority-root":
                assert isinstance(selected, pale_python.EffectiveProfileAuthorityRootSelection)
                assert selected.authority_root_id == expected["authorityRootId"]
            elif expected["kind"] == "semantic-resource":
                assert isinstance(selected, pale_python.EffectiveProfileSemanticResourceSelection)
                assert selected.repository_id == expected["repositoryId"]
                assert selected.resource_id == expected["resourceId"]
                assert selected.resource_kind == pale_python.EffectiveProfileResourceKind.Role
            else:
                raise AssertionError("fixture emitted an unexamined selection")
        elif row["kind"] == "machine-realization":
            assert proof.realization == pale_python.EffectiveProfileNativeRealization.Unknown
        elif row["kind"] == "finding":
            assert isinstance(proof.finding, pale_python.EffectiveProfileNativeRealizationUnknownFinding)
            assert proof.finding.machine_id == row["finding"]["machineId"]
        else:
            assert not hasattr(proof, "reason_steps") and not hasattr(proof, "selection")
assert seen_evidence == set(evidence_classes)
retained_reason = retained_evidence.reason_steps[0]
retained_string = retained_reason.contract_id
assert retained_string == "origin-receipt"
del effective
gc.collect()
assert retained_selection.run_policy_id == retained_policy
try:
    retained_selection.run_policy_id = "changed"
except AttributeError:
    pass
else:
    raise AssertionError("copied selections must remain frozen")
del retained_selection
gc.collect()
assert retained_policy
retained_detail = retained_entry.detail
assert isinstance(retained_detail, pale_python.EffectiveProfileResponsibilityDetail)
retained_participant = retained_detail.participant
retained_contract = retained_detail.contract_id
assert isinstance(retained_contract, str)
try:
    retained_detail.contract_id = "changed"
except AttributeError:
    pass
else:
    raise AssertionError("retained profile entries must be frozen")
del retained_entry
gc.collect()
assert retained_detail.contract_id == retained_contract
assert type(retained_detail.participant) is type(retained_participant)
del retained_detail, retained_participant
gc.collect()
assert isinstance(retained_contract, str) and retained_contract
assert retained_evidence.reason_steps[0].contract_id == retained_string
try:
    retained_reason.contract_id = "changed"
except AttributeError:
    pass
else:
    raise AssertionError("retained profile values must be frozen")
del retained_reason, retained_evidence
gc.collect()
assert retained_string == "origin-receipt"
compilation = api.compile_one_shot(pale_python.OneShotCompilationRequest(
    project_id=one_shot["selectors"]["project_id"],
    execution_workspace_id=one_shot["selectors"]["execution_workspace_id"],
    machine_id=one_shot["selectors"]["machine_id"],
    private_inputs=private_inputs,
    package_documents=package_documents,
    workspace=pale_python.OneShotWorkspaceInputs("pale-workspace.toml", workspace_documents),
))
summary = api.one_shot_summary(compilation)
assert {
    "project_id": summary.project_id,
    "execution_workspace_id": summary.execution_workspace_id,
    "profile_digest": summary.profile_digest,
    "instance_digest": summary.instance_digest,
    "one_shot_compilation_digest": summary.one_shot_compilation_digest,
    "schema_manifest_sha256": summary.schema_manifest_sha256,
    "schema_set_sha256": summary.schema_set_sha256,
    "repository_ids": summary.repository_ids,
    "canonical_sha256": hashlib.sha256(summary.canonical_bytes).hexdigest(),
    "canonical_length": len(summary.canonical_bytes),
} == one_shot["summary"]
prior = pale_python.OneShotPriorMaterialization.unowned()
observation = pale_python.OneShotObservationRequest(one_shot["selectors"]["repository_id"], pale_python.OneShotPlanKind.Apply, prior)
targets = api.one_shot_observation_targets(compilation, observation)
assert targets == one_shot["observation_targets"]
baseline = [pale_python.BaselineEntry.absent(target) for target in targets]
planning = pale_python.OneShotPlanningRequest(
    pale_python.OneShotObservationRequest(one_shot["selectors"]["repository_id"], pale_python.OneShotPlanKind.Apply, pale_python.OneShotPriorMaterialization.unowned()), baseline,
)
plan = api.plan_one_shot(compilation, planning)
assert {
    "repository_id": plan.repository_id,
    "plan_digest": plan.plan_digest,
    "one_shot_compilation_digest": plan.one_shot_compilation_digest,
    "operation_count": plan.operation_count,
    "deferred_effect_count": plan.deferred_effect_count,
    "canonical_sha256": hashlib.sha256(plan.canonical_bytes).hexdigest(),
    "canonical_length": len(plan.canonical_bytes),
} == one_shot["plan"]
bound = api.bind_one_shot_machine_effects(plan, [])
bound_bytes = bound.canonical_bytes
assert bound.repository_id == one_shot["selectors"]["repository_id"]
assert bound_bytes
bound_value = json.loads(bound_bytes)
assert bound_value["surface_plan"]["plan_digest"] == plan.plan_digest
admitted_plan = api.admit_one_shot_saved_plan(bound, bound_bytes)
assert admitted_plan.repository_id == one_shot["selectors"]["repository_id"]
assert admitted_plan.plan_digest == bound_value["plan_digest"]

repository_admission = json.loads((fixture_root / "repository-admission.json").read_text(encoding="utf-8"))
admitted = api.admit_repository_native_actions(
    (fixture_root / "repository-catalog.json").read_bytes(),
    repository_admission["profile_digest"],
    repository_admission["instance_digest"],
    repository_admission["repository_id"],
)
assert admitted.profile_digest == repository_admission["profile_digest"]
assert admitted.instance_digest == repository_admission["instance_digest"]
assert admitted.repository_id == repository_admission["repository_id"]
assert [descriptor.action_id for descriptor in admitted.descriptors()] == action["descriptor_ids"]

package_inspection = api.inspect_package_distribution(
    [
        Document(
            f"engine/{name}",
            (one_shot_root / "packages/engine" / name).read_bytes(),
        )
        for name in ["package.toml", "role.md", "roles.toml"]
    ]
)
assert [(package.id, package.version) for package in package_inspection.packages] == [
    ("example.engine", "1.0.0")
]
assert [
    (package["id"], package["version"])
    for package in json.loads(package_inspection.canonical_bytes)["packages"]
] == [("example.engine", "1.0.0")]

resident_admission = api.admit_resident_hook(
    pale_python.ResidentHookRequest(
        event="session-start",
        workspace="workspace-a",
        repository="repository-a",
        provider_binding="provider-binding-claude-code",
        provider_binding_identity="ec03a4e4096f39aa0a8c2e3fe9b69705b0ff620963137a0558224b9826a4f3b6",
        payload=b'{"hook_event_name":"SessionStart","session_id":"native-session"}',
    )
)
try:
    api.resident_hook_transport_unavailable(resident_admission)
except pale_python.PaleDiagnosticError as error:
    structured = json.loads(error.diagnostic_bytes)
    assert error.diagnostic.code == "resident_hook_transport_unavailable"
    assert error.diagnostic.effect == pale_python.DiagnosticEffectCertainty.NotStarted
    assert structured["code"] == "resident_hook_transport_unavailable"
    assert structured["effect"] == "not_started"
else:
    raise AssertionError("missing resident carrier must fail closed")

compiled_identity = api.compiled_interface_identity()
assert compiled_identity.version == "0.0.1"
assert json.loads(compiled_identity.canonical_bytes)["protocol"] == compiled_identity.protocol

with tempfile.TemporaryDirectory(prefix="pale-python-abi-") as temporary:
    root = pathlib.Path(temporary).resolve()
    for name in [
        "composition.toml",
        "pale-workspace.toml",
        "workspace-resolution.lock.toml",
        "workspace-universe.lock.toml",
    ]:
        (root / name).write_bytes((one_shot_root / "workspace" / name).read_bytes())
    for name in ["capabilities.toml", "distribution-lock.toml"]:
        (root / name).write_bytes((one_shot_root / "private" / name).read_bytes())
    package_root = root / "distribution/packages/engine"
    package_root.mkdir(parents=True)
    for name in ["package.toml", "role.md", "roles.toml"]:
        (package_root / name).write_bytes((one_shot_root / "packages/engine" / name).read_bytes())
    checkout = root / "checkout"
    authority = root / "authority"
    lineage = root / "lineage"
    for directory in [checkout, authority, lineage, checkout / ".git/info"]:
        directory.mkdir(parents=True, exist_ok=True)
    (checkout / ".git/info/exclude").write_text("# user-owned exclusion\n", encoding="utf-8")
    executable = _running_executable()
    machine = f'''[machine]
id = "machine"
schema_version = 1
distribution_root = {json.dumps(str(root / "distribution"))}

[[repository_checkouts]]
repository_id = "engine"
checkout_root = {json.dumps(str(checkout))}

[authority_roots]
authority = {json.dumps(str(authority))}

[materialization_lineage]
root = {json.dumps(str(lineage))}
max_route_records_per_collection = 4096

[surface_executable]
exec = {json.dumps(str(executable))}
digest = {json.dumps(hashlib.sha256(executable.read_bytes()).hexdigest())}
version = {json.dumps(compiled_identity.version)}
protocol = {json.dumps(compiled_identity.protocol)}
'''
    machine_path = root / "machines/machine.toml"
    machine_path.parent.mkdir(parents=True)
    machine_path.write_text(machine, encoding="utf-8")

    exercise_local_configuration_improvement(root)

    local_request = pale_python.LocalOneShotCompilationRequest(
        workspace_manifest=str(root / "pale-workspace.toml"),
        project_id=one_shot["selectors"]["project_id"],
        execution_workspace_id=one_shot["selectors"]["execution_workspace_id"],
        machine_id=one_shot["selectors"]["machine_id"],
    )
    local_compilation = api.compile_local_one_shot(local_request)
    local_summary = local_compilation.summary()
    assert local_summary.project_id == one_shot["selectors"]["project_id"]
    assert local_summary.execution_workspace_id == one_shot["selectors"]["execution_workspace_id"]
    assert local_summary.repository_ids == [one_shot["selectors"]["repository_id"]]
    local_packages = api.inspect_local_package_distribution(
        pale_python.LocalPackageDistributionRequest(
            workspace_manifest=str(root / "pale-workspace.toml"),
            project_id=one_shot["selectors"]["project_id"],
            execution_workspace_id=one_shot["selectors"]["execution_workspace_id"],
            machine_id=one_shot["selectors"]["machine_id"],
        )
    )
    assert [(package.id, package.version) for package in local_packages.packages] == [
        ("example.engine", "1.0.0")
    ]
    action_path = root / "input/action.json"
    action_path.parent.mkdir(parents=True)
    action_path.write_text(action["tagged_input"], encoding="utf-8")
    captured_action = api.capture_local_native_action_request(
        pale_python.LocalNativeActionInputRequest(
            action["action_id"], str(root), "input/action.json"
        )
    )
    assert bytes(api.invoke_captured_native_action(captured_action)) == tagged_execution
    local_status = api.inspect_local_repository_status(
        local_compilation, one_shot["selectors"]["repository_id"]
    )
    assert not local_status.is_clean
    assert local_status.drift
    local_plan = api.plan_local_one_shot(
        local_compilation,
        pale_python.LocalOneShotPlanRequest(
            one_shot["selectors"]["repository_id"], pale_python.OneShotPlanKind.Apply
        ),
    )
    assert local_plan.counts().create > 0
    plans = root / "plans"
    publication = api.publish_local_one_shot_plan(local_plan, str(plans))
    published = (plans / publication.relative_target).read_bytes()
    assert published.startswith(b'{\n  "') and published.endswith(b"\n")
    evidence = root / "evidence"
    admitted_run = api.admit_local_materialization_run(
        local_compilation,
        pale_python.LocalMaterializationAdmissionRequest(
            pale_python.OneShotPlanKind.Apply, str(plans), str(evidence)
        ),
    )
    assert [target.repository_id for target in admitted_run.targets()] == [
        one_shot["selectors"]["repository_id"]
    ]
    try:
        api.inspect_local_materialization_reconciliation(admitted_run)
    except pale_python.PaleDiagnosticError as error:
        assert error.diagnostic.code == "local_materialization_phase_rejected"
        assert error.diagnostic.effect == pale_python.DiagnosticEffectCertainty.NotApplicable
    else:
        raise AssertionError("pre-settlement reconciliation must fail closed")
    verified_identity = api.verify_current_executable_binding(local_compilation)
    assert verified_identity.canonical_bytes == compiled_identity.canonical_bytes
    settled = api.settle_local_materialization_run(admitted_run)
    assert settled.outcome == pale_python.LocalMaterializationRunOutcome.Succeeded
    assert settled.succeeded_target_count == 1
    assert len(settled.targets) == 1
    assert settled.targets[0].changed_surface_count > 0
    receipt = (evidence / "run-result.apply.json").read_bytes()
    assert receipt.startswith(b'{\n  "') and receipt.endswith(b"\n")
    assert api.inspect_local_repository_status(
        local_compilation, one_shot["selectors"]["repository_id"]
    ).is_clean

executed = [
    fixture["capability_id"],
    *contract["capability_ids"],
    "package-distribution.inspection",
    "authoring.ingress-limits",
    "authoring.standalone-validation",
    "authoring.standalone-inspection",
    "workspace.portable-compilation",
    "workspace.portable-authoring-inspection",
    "workspace.portable-authoring-navigation",
    "workspace.portable-authoring-occurrences",
    "workspace.portable-subject-comparison",
    "workspace.portable-comparison-subjects",
    "workspace.portable-direct-dependencies",
    *portable["capability_ids"][1:],
    "workspace.effective-profile-compilation",
    "workspace.effective-profile-inspection",
    "workspace.effective-profile-explanation",
    "workspace.effective-profile-authoring-inspection",
    "workspace.effective-profile-authoring-navigation",
    "workspace.effective-profile-authoring-join",
    "workspace.effective-profile-authoring-occurrences",
    "workspace.effective-profile-authoring-pages",
    "workspace.configuration-improvement",
    "observation.snapshot",
    "observation.page",
    observation_profile_capability,
    "observation.delta",
    "observation.causal-path",
    "observation.visual-lens",
    *action["capability_ids"],
    repository_admission["capability_id"],
    "resident-hook.admission",
    *one_shot["capability_ids"],
    "one-shot.machine-effect-binding",
    "one-shot.saved-plan-admission",
    "local.native-action-input-capture",
    "local.one-shot-compilation",
    "local.package-distribution-inspection",
    "local.repository-status",
    "local.one-shot-planning",
    "local.one-shot-plan-publication",
    "local.materialization-run-admission",
    "local.materialization-run-settlement",
    "local.materialization-reconciliation-inspection",
    "local.compiled-interface-identity",
    "local.current-executable-binding-verification",
    "local.control-storage-inspection",
    "local.configuration-improvement-preview",
]
witnessed = list(dict.fromkeys(executed))
assert [capability for capability in witnessed if capability not in api.capability_ids] == []
assert [capability for capability in api.capability_ids if capability not in witnessed] == []
print(json.dumps({"executed": executed, "exported": api.capability_ids,
                  "comparison_subjects": discovery["comparison_subjects"],
                  "direct_dependencies": discovery["direct_dependencies"]}, indent=2))
