"""Compare real admitted subjects through the installed Python wheel."""

import gc
import json

import pypale as pale

from comparison_subjects import exercise_comparison_subjects
from proof_dependencies import exercise_proof_dependencies


_FACT_FIELDS = {
    "source_path": "sourcePath", "source_digest": "sourceDigest", "byte_len": "byteLen",
    "component_digest": "componentDigest", "module_digest": "moduleDigest",
    "value_digest": "valueDigest", "surface_digest": "surfaceDigest",
    "closure_digest": "closureDigest", "resolution_digest": "resolutionDigest",
    "universe_digest": "universeDigest", "workspace_semantic_ir_digest": "workspaceSemanticIrDigest",
}
_FACT_KINDS = {
    "source_content": pale.WorkspaceFactKind.SourceContent,
    "component": pale.WorkspaceFactKind.Component,
    "module": pale.WorkspaceFactKind.Module,
    "declaration": pale.WorkspaceFactKind.Declaration,
    "import_surface": pale.WorkspaceFactKind.ImportSurface,
    "extension": pale.WorkspaceFactKind.Extension,
    "effective_policy": pale.WorkspaceFactKind.EffectivePolicy,
    "responsibility_closure": pale.WorkspaceFactKind.ResponsibilityClosure,
    "organizational_translation_closure": pale.WorkspaceFactKind.OrganizationalTranslationClosure,
    "experience_composition": pale.WorkspaceFactKind.ExperienceComposition,
    "resource": pale.WorkspaceFactKind.Resource,
    "resolution_lock": pale.WorkspaceFactKind.ResolutionLock,
    "universe_lock": pale.WorkspaceFactKind.UniverseLock,
    "semantic_ir": pale.WorkspaceFactKind.SemanticIr,
}
_SUBJECT_FIELDS = (
    "kind", "workspace_id", "source_id", "component_id", "module_id", "owner_module_id",
    "declaration_namespace_id", "declaration_id", "importer_module_id", "upstream_module_id",
    "upstream_release_id", "export_id", "source_namespace_id", "local_namespace_id",
    "extension_namespace_id", "resource_id",
)


def _reject(call, expected):
    try:
        call()
    except expected:
        return
    raise AssertionError(f"expected {expected}")


def _fact_payload(fact):
    return {wire: getattr(fact, field) for field, wire in _FACT_FIELDS.items()
            if getattr(fact, field) is not None}


def _assert_fact(fact, wire):
    assert isinstance(fact, pale.WorkspaceFact)
    assert fact.kind == _FACT_KINDS[wire["kind"]]
    assert _fact_payload(fact) == {key: value for key, value in wire.items() if key != "kind"}


def _answer(owner, subject, variant, comparison=None):
    result = owner.compare_subject(subject)
    assert isinstance(result, pale.WorkspaceSubjectComparison)
    assert isinstance(result.canonical_bytes, bytes)
    wire = json.loads(result.canonical_bytes)
    assert result.receipt_digest == owner.receipt_digest == wire["receiptDigest"]
    copied_subject = result.subject()
    assert all(getattr(copied_subject, field) == getattr(subject, field) for field in _SUBJECT_FIELDS)
    values = result.values
    assert isinstance(values, variant)
    memberships = {
        pale.AbsentWorkspaceSubjectValues: "absent",
        pale.PriorOnlyWorkspaceSubjectValues: "prior_only",
        pale.CurrentOnlyWorkspaceSubjectValues: "current_only",
        pale.SharedWorkspaceSubjectValues: "shared",
    }
    assert wire["values"]["membership"] == memberships[variant]
    for field, key in (("prior", "priorFact"), ("current", "currentFact")):
        if key in wire["values"]:
            _assert_fact(getattr(values, field), wire["values"][key])
        else:
            assert not hasattr(values, field)
    if comparison is None:
        assert not hasattr(values, "comparison") and "comparison" not in wire["values"]
    else:
        assert values.comparison == comparison
        assert wire["values"]["comparison"] == (
            "equal" if comparison == pale.WorkspaceValueComparison.Equal else "changed")
    before = result.canonical_bytes
    mutable_copy = bytearray(before)
    mutable_copy[0] ^= 1
    _reject(lambda: setattr(result, "receipt_digest", "forged"), AttributeError)
    _reject(lambda: setattr(values, "comparison", pale.WorkspaceValueComparison.Changed), AttributeError)
    _reject(lambda: setattr(copied_subject, "source_id", "forged"), AttributeError)
    if hasattr(values, "prior"):
        _reject(lambda: setattr(values.prior, "source_digest", "forged"), AttributeError)
    assert result.canonical_bytes == before
    assert owner.compare_subject(copied_subject).canonical_bytes == before
    return result


def exercise_subject_comparison(api, portable, documents):
    root_label = portable["root_manifest"]
    root_bytes = dict(documents)[root_label]

    def compile_root(contents, label=root_label, extra=()):
        return api.compile_portable_workspace(label, [
            (label, contents) if source_label == root_label else (source_label, body)
            for source_label, body in documents
        ] + list(extra))

    prior = compile_root(root_bytes)
    current = compile_root(root_bytes + b"\n# Source-only review change.\n")
    assert prior.semantic_ir_digest == current.semantic_ir_digest
    pair = api.compare_portable_workspaces(prior, current)
    assert pair.prior_principal_universe_digest == prior.principal_universe_digest
    assert pair.current_principal_universe_digest == current.principal_universe_digest
    assert pair.prior_semantic_ir_digest == prior.semantic_ir_digest
    assert pair.current_semantic_ir_digest == current.semantic_ir_digest
    roots = pair.change_roots(0, 256)
    assert roots.next_offset is None and len(roots.entries) == 1
    source_root = roots.entries[0]
    source = source_root.subject()
    changed = _answer(pair, source, pale.SharedWorkspaceSubjectValues, pale.WorkspaceValueComparison.Changed)
    assert changed.values.prior.source_digest != changed.values.current.source_digest
    assert _fact_payload(changed.values.prior) == _fact_payload(source_root.prior_fact())
    assert _fact_payload(changed.values.current) == _fact_payload(source_root.current_fact())
    assert json.loads(changed.canonical_bytes)["subject"]["sourceId"] == source.source_id

    impacts = pair.impacts(0, 256)
    assert impacts.next_offset is None
    workspace = next(entry for entry in impacts.entries if entry.subject().kind == pale.WorkspaceSubjectKind.Workspace)
    assert workspace.state == pale.WorkspaceImpactState.Affected
    equal = _answer(pair, workspace.subject(), pale.SharedWorkspaceSubjectValues, pale.WorkspaceValueComparison.Equal)
    assert equal.values.prior.workspace_semantic_ir_digest == equal.values.current.workspace_semantic_ir_digest
    assert equal.values.current.workspace_semantic_ir_digest == current.semantic_ir_digest
    assert pair.explain(equal.subject()).state == pale.WorkspaceExplanationState.Affected
    assert json.loads(equal.canonical_bytes)["subject"]["workspaceId"] == prior.workspace_id
    for impact in impacts.entries:
        answer = pair.compare_subject(impact.subject())
        assert isinstance(answer.values, pale.SharedWorkspaceSubjectValues)
        wire = json.loads(answer.canonical_bytes)["values"]
        _assert_fact(answer.values.prior, wire["priorFact"])
        _assert_fact(answer.values.current, wire["currentFact"])

    # An explicit empty composition adds captured source without selecting a provider.
    composition = b'''schema = "pale/workspace-composition-source.v1"
schema_version = 1
[declarations]
responsibility_contracts = []
participant_provider_manifests = []
provider_bindings = []
entry_contracts = []
request_translations = []
result_translations = []
termination_contracts = []
settlement_contracts = []
continuity_contracts = []
edge_contracts = []
selected_experience_paths = []
'''
    selected_root = root_bytes.replace(b"experience_compositions = []", b'experience_compositions = ["composition.toml"]')
    assert selected_root != root_bytes
    selected = compile_root(selected_root, extra=[("composition.toml", composition)])
    forward = api.compare_portable_workspaces(prior, selected)
    reverse = api.compare_portable_workspaces(selected, prior)
    added_subject = next(entry.subject() for entry in forward.change_roots(0, 256).entries
                         if entry.kind == pale.WorkspaceChangeKind.Added)
    added = _answer(forward, added_subject, pale.CurrentOnlyWorkspaceSubjectValues)
    removed = _answer(reverse, added_subject, pale.PriorOnlyWorkspaceSubjectValues)
    assert _fact_payload(added.values.current) == _fact_payload(removed.values.prior)
    assert added.values.current.source_path == "composition.toml"
    absent = _answer(pair, added_subject, pale.AbsentWorkspaceSubjectValues)
    assert json.loads(absent.canonical_bytes)["values"] == {"membership": "absent"}
    assert pair.explain(added_subject).state == pale.WorkspaceExplanationState.NotPresent

    fresh = compile_root(root_bytes)
    discovery = exercise_comparison_subjects(api, prior, selected, added_subject, fresh)
    discovery.update(exercise_proof_dependencies(api, prior, added_subject, fresh, _assert_fact))

    def retained_discovery():
        owner_api = pale.Pale()
        owner = owner_api.compile_portable_workspace(root_label, documents)
        comparison = owner_api.compare_portable_workspaces(owner, owner)
        subject = next(entry.subject() for entry in comparison.impacts(0, 256).entries
                       if entry.subject().kind == pale.WorkspaceSubjectKind.UniverseLock)
        return comparison.subjects(1), owner.direct_dependencies(subject, 1)

    retained_union, retained_inputs = retained_discovery()
    retained_bytes = retained_union.canonical_bytes, retained_inputs.canonical_bytes
    gc.collect()
    assert (retained_union.canonical_bytes, retained_inputs.canonical_bytes) == retained_bytes
    assert retained_union.subjects() and retained_union.comparison().receipt_digest
    assert isinstance(retained_union.continuation, pale.WorkspaceComparisonSubjectCursor)
    assert retained_inputs.root().fact().kind == pale.WorkspaceFactKind.UniverseLock
    assert retained_inputs.dependencies()
    assert isinstance(retained_inputs.continuation, pale.PortableWorkspaceDependencyCursor)

    renamed = compile_root(root_bytes, "renamed.toml")
    renamed_pair = api.compare_portable_workspaces(prior, renamed)
    renamed_result = _answer(renamed_pair, source, pale.SharedWorkspaceSubjectValues, pale.WorkspaceValueComparison.Changed)
    assert renamed_result.values.prior.source_digest == renamed_result.values.current.source_digest
    assert renamed_result.values.prior.byte_len == renamed_result.values.current.byte_len
    assert renamed_result.values.prior.source_path == root_label
    assert renamed_result.values.current.source_path == "renamed.toml"

    for invalid in (None, {}, "workspace-manifest", prior, changed, source_root):
        _reject(lambda: pair.compare_subject(invalid), TypeError)
    for cls in (pale.WorkspaceSubjectComparison, pale.AbsentWorkspaceSubjectValues,
                pale.PriorOnlyWorkspaceSubjectValues, pale.CurrentOnlyWorkspaceSubjectValues,
                pale.SharedWorkspaceSubjectValues):
        _reject(lambda: cls(), TypeError)
        _reject(lambda: cls.__new__(cls), TypeError)

    # Comparisons retain producer compilations; answers retain their own selected facts.
    del prior, current, selected, renamed, fresh
    gc.collect()
    assert pair.compare_subject(source).canonical_bytes == changed.canonical_bytes
    del pair, forward, reverse, renamed_pair, roots, source_root, impacts, workspace
    gc.collect()
    assert equal.values.current.workspace_semantic_ir_digest
    assert changed.values.current.source_digest
    assert added.values.current.source_path == removed.values.prior.source_path
    assert isinstance(absent.values, pale.AbsentWorkspaceSubjectValues)
    return discovery
