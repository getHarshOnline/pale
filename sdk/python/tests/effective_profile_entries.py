"""Actual union payloads compared with the same retained canonical SDK pages."""

import json
import re

import pypale as pale


DETAIL_VARIANTS = (
    "DeclaredSelection", "Responsibility", "Selection", "CapabilityNarrowing",
    "GoverningCeiling", "Repository", "ProviderRepository", "ExecutionPath",
    "Operation", "Capability", "MachineRealization", "Finding",
)
PARTICIPANT_VARIANTS = ("Interface", "Application", "Engine", "Capability", "TerminalLeaf", "Plugin", "Skill")
RESPONSIBILITY_VARIANTS = (
    "Communication", "OrganizationalWork", "CrmBusinessState", "EngineeringExecution",
    "ContextMemory", "TerminalHarnessExecution", "NativeEventPublication",
    "NativeEffectReadBack", "Extension",
)
SUBJECT_VARIANTS = (
    "Source", "Component", "Module", "Declaration", "Import", "Extension", "Policy",
    "ResponsibilityClosure", "OrganizationalTranslationClosure", "ExperienceComposition",
    "Resource", "ResolutionLock", "UniverseLock",
)
SELECTION_VARIANTS = (
    "Package", "NormativePack", "SemanticResource", "RunPolicy", "Session",
    "Lifecycle", "AuthorityRoot", "DisclosureCeiling",
)
DECLARED_CLASSES = (
    "EffectiveProfileDeclaredExactSelector", "EffectiveProfileDeclaredAllFromPackageSelector",
    "EffectiveProfileDeclaredResourceSelection", "EffectiveProfileDeclaredModeDefaultSelection",
)


def _title(tag):
    return "".join(part.capitalize() for part in tag.split("-"))


def _field(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def _assert_projection(actual, expected, family=None):
    """Test-only canonical-value oracle, not a consumer reconstruction API."""
    if isinstance(expected, dict):
        if family is not None:
            tag = expected["kind"]
            if family == "subject":
                name = "WholeWorkspaceSubjectValue" if tag == "workspace" else f"Workspace{_title(tag)}SubjectValue"
            elif family in ("DeclaredSelection", "DeclaredSelector"):
                suffix = "Selection" if family == "DeclaredSelection" else "Selector"
                name = f"EffectiveProfileDeclared{_title(tag)}{suffix}"
            elif family == "RequirementDisposition":
                name = f"{_title(tag)}RequirementDisposition"
            else:
                name = f"EffectiveProfile{_title(tag)}{family}"
            assert isinstance(actual, getattr(pale, name))
            assert {name for name in dir(actual) if not name.startswith("_")} == {
                _field(name) for name in expected if name != "kind"
            }
        for name, value in expected.items():
            if name == "kind" and family is not None:
                continue
            nested = {
                "participant": "Participant", "responsibilities": "Responsibility", "subject": "subject",
                "reasonSteps": "Reason", "requirementDisposition": "RequirementDisposition",
                "disposition": "RequirementDisposition", "providerDisposition": "ProviderDisposition",
                "finding": "Finding",
            }.get(name)
            if family in ("Detail", "Evidence") and expected["kind"] in ("selection", "effective-selection") and name == "selection":
                nested = "Selection"
            if family in ("Detail", "Evidence") and expected["kind"] == "declared-selection" and name == "selection":
                nested = "DeclaredSelection"
            if family == "DeclaredSelection" and name == "selector":
                nested = "DeclaredSelector"
            _assert_projection(getattr(actual, _field(name)), value, nested)
    elif isinstance(expected, list):
        assert isinstance(actual, list) and len(actual) == len(expected)
        for value, wanted in zip(actual, expected):
            _assert_projection(value, wanted, family)
    elif isinstance(expected, str) and not isinstance(actual, str):
        assert actual == getattr(type(actual), _title(expected))
    else:
        assert actual == expected


def _assert_entry(entry, expected):
    assert isinstance(entry, pale.EffectiveProfileEntry)
    assert entry.explanation_handle == expected["explanationHandle"]
    assert {name for name in dir(entry) if not name.startswith("_")} == {"detail", "explanation_handle"}
    detail = entry.detail
    _assert_projection(detail, {key: value for key, value in expected.items() if key != "explanationHandle"}, "Detail")
    assert detail is not entry.detail
    return detail


def exercise_effective_profile_entries(profile, workspace, identity):
    classes = ["EffectiveProfileEntry", "EffectiveProfileProviderBindingRef", "WholeWorkspaceSubjectValue"]
    classes += [f"EffectiveProfile{name}Detail" for name in DETAIL_VARIANTS]
    classes += [f"EffectiveProfile{name}Participant" for name in PARTICIPANT_VARIANTS]
    classes += [f"EffectiveProfile{name}Responsibility" for name in RESPONSIBILITY_VARIANTS]
    classes += [f"Workspace{name}SubjectValue" for name in SUBJECT_VARIANTS]
    classes += [f"EffectiveProfile{name}Selection" for name in SELECTION_VARIANTS]
    classes += list(DECLARED_CLASSES)
    assert len(classes) == 56
    for name in classes:
        cls = getattr(pale, name)
        assert cls.__name__ == name
        for construct in (lambda: cls(), lambda: cls.__new__(cls)):
            try:
                construct()
            except TypeError:
                pass
            else:
                raise AssertionError(f"{name} must remain SDK-created at runtime")
    for name in (
        "EffectiveProfileEntryKind", "EffectiveProfileParticipant", "EffectiveProfileParticipantKind",
        "EffectiveProfileResponsibility", "EffectiveProfileResponsibilityKind", "WorkspaceSubjectValue",
        "EffectiveProfileSelection", "EffectiveProfileSelectionKind",
        "EffectiveProfileDeclaredSelection", "EffectiveProfileDeclaredSelectionKind",
        "EffectiveProfileDeclaredSelector", "EffectiveProfileDeclaredSelectorKind",
    ):
        assert not hasattr(pale, name)
    for name in ("EffectiveProfileEffectClass", "EffectiveProfileRepositoryAccess"):
        assert getattr(pale, name).__name__ == name

    rows = {}
    seen = set()
    seen_selections = set()
    retained = None
    for section in (
        pale.EffectiveProfileSection.IntendedUseCase,
        pale.EffectiveProfileSection.EffectiveConfiguration,
        pale.EffectiveProfileSection.CapabilitiesAndOperations,
        pale.EffectiveProfileSection.MachineRealization,
        pale.EffectiveProfileSection.GapsAndExplanations,
    ):
        page = profile.inspect(section, 0, 256)
        canonical = json.loads(page.canonical_bytes)
        assert page.entry_count == len(page.entries) == len(canonical["entries"])
        assert page.next_offset is None
        assert page.offset == canonical["offset"]
        for entry, expected in zip(page.entries, canonical["entries"]):
            detail = _assert_entry(entry, expected)
            rows[entry.explanation_handle] = expected
            seen.add(expected["kind"])
            if isinstance(detail, pale.EffectiveProfileSelectionDetail):
                seen_selections.add(expected["selection"]["kind"])
                _assert_projection(profile.explain(entry.explanation_handle).evidence.selection, expected["selection"], "Selection")
            if isinstance(detail, pale.EffectiveProfileResponsibilityDetail):
                retained = entry
                detail.responsibilities.clear()
                assert len(detail.responsibilities) == len(expected["responsibilities"])
                assert not isinstance(detail.subject, pale.WorkspaceSubject)
        page.entries.clear()
        assert len(page.entries) == page.entry_count
        empty = profile.inspect(section, 2**32 - 1, 256)
        assert empty.entries == [] and empty.entry_count == 0
        assert empty.offset == 2**32 - 1 and empty.next_offset is None
    assert seen == {"responsibility", "execution-path", "selection", "repository", "machine-realization", "finding"}
    assert retained is not None
    assert seen_selections == {"package", "run-policy", "authority-root", "semantic-resource"}

    joined = profile.inspect_effective_authoring(identity)
    assert joined.entries and not joined.truncated
    assert not hasattr(joined, "explanation_handles")
    for entry in joined.entries:
        detail = _assert_entry(entry, rows[entry.explanation_handle])
        assert isinstance(detail, pale.EffectiveProfileSelectionDetail)
        assert isinstance(detail.selection, pale.EffectiveProfileRunPolicySelection)
        assert detail.selection.run_policy_id == "standard"
        assert profile.explain(entry.explanation_handle).handle == entry.explanation_handle
    joined.entries.clear()
    assert joined.entries

    portable_navigation = workspace.navigate_authoring("pale-workspace.toml", [
        pale.AuthoringInstancePathSegment.field("workspace"),
        pale.AuthoringInstancePathSegment.field("desired_topology"),
        pale.AuthoringInstancePathSegment.field("projects"),
        pale.AuthoringInstancePathSegment.index(0),
        pale.AuthoringInstancePathSegment.field("run_policy"),
    ])
    assert portable_navigation.identity is not None
    portable_join = profile.inspect_effective_authoring(portable_navigation.identity)
    assert [entry.explanation_handle for entry in portable_join.entries] == [entry.explanation_handle for entry in joined.entries]
    for entry in portable_join.entries:
        _assert_entry(entry, rows[entry.explanation_handle])
    for field in (
        "workspace_id", "project_id", "execution_workspace_id", "machine_id",
        "instance_digest", "profile_digest", "package_semantic_ir_digest",
        "workspace_semantic_ir_digest", "workspace_admission_evidence_digest",
        "composition_plan_digest", "selected_topology_digest",
    ):
        assert getattr(joined.coordinates, field) == getattr(profile.coordinates(), field)
        assert getattr(portable_join.coordinates, field) == getattr(joined.coordinates, field)
    return retained, rows
