"""Direct proof-input payload and custody laws through the installed wheel."""

import gc
import json

import pypale as pale


def exercise_proof_dependencies(api, prior, absent_subject, fresh, assert_fact):
    reference = api.compare_portable_workspaces(prior, prior)
    subjects_page = reference.impacts(0, 256)
    assert subjects_page.next_offset is None
    subjects = [entry.subject() for entry in subjects_page.entries]
    lock = next(subject for subject in subjects if subject.kind == pale.WorkspaceSubjectKind.UniverseLock)
    leaf = next(subject for subject in subjects if subject.kind == pale.WorkspaceSubjectKind.Source)

    def wire_subject(subject):
        return json.loads(reference.compare_subject(subject).canonical_bytes)["subject"]

    def check_step(step, wire):
        assert isinstance(step, pale.WorkspaceCausalStep)
        assert step.node_digest == wire["nodeDigest"]
        assert wire_subject(step.subject()) == wire["subject"]
        assert_fact(step.fact(), wire["fact"])
        rule_name = "".join(part.title() for part in wire["rule"].split("_"))
        assert step.rule == getattr(pale.WorkspaceDerivationRule, rule_name)
        provenance = step.provenance()
        provenance_wire = wire["provenance"]
        kinds = {"source": pale.WorkspaceProvenanceKind.Source,
                 "compilation": pale.WorkspaceProvenanceKind.Compilation}
        assert provenance.kind == kinds[provenance_wire["kind"]]
        assert provenance.source_id == provenance_wire.get("sourceId")
        for field in ("module_id", "release_id", "module_digest", "importer_module_id",
                      "upstream_module_id", "upstream_release_id", "export_id",
                      "source_namespace_id", "local_namespace_id", "extension_namespace_id",
                      "target_namespace_id", "target_declaration_id"):
            assert getattr(provenance, field) is None

    def check_page(page, subject):
        assert isinstance(page, pale.PortableWorkspaceDependencyPage)
        wire = json.loads(page.canonical_bytes)
        assert page.principal_universe_digest == wire["principalUniverseDigest"] == prior.principal_universe_digest
        assert page.semantic_ir_digest == wire["semanticIrDigest"] == prior.semantic_ir_digest
        assert wire_subject(page.subject()) == wire["subject"] == wire_subject(subject)
        root = page.root()
        if root is None:
            assert wire["root"] is None
        else:
            check_step(root, wire["root"])
        rows = page.dependencies()
        assert len(rows) == len(wire["dependencies"])
        for step, raw in zip(rows, wire["dependencies"]):
            check_step(step, raw)
        assert page.is_complete == wire["isComplete"] == (page.continuation is None)
        return wire

    terminal = None
    for subject in [*subjects, absent_subject]:
        whole = prior.direct_dependencies(subject, 256)
        expected = check_page(whole, subject)
        assert whole.is_complete
        for limit in (1, 2, 0, 256, 65535):
            cursor = None
            actual = []
            for _ in range(256):
                page = prior.direct_dependencies(subject, limit, cursor)
                wire = check_page(page, subject)
                assert wire["root"] == expected["root"]
                assert len(wire["dependencies"]) <= min(limit or 64, 256)
                actual.extend(wire["dependencies"])
                cursor = page.continuation
                if cursor is None:
                    terminal = page
                    break
            else:
                raise AssertionError("dependency enumeration did not terminate")
            assert actual == expected["dependencies"]
            digests = [step["nodeDigest"] for step in actual]
            assert digests == sorted(set(digests))
    # This real empty-workspace fixture's universe lock consumes two direct inputs.
    lock_page = prior.direct_dependencies(lock, 256)
    assert len(lock_page.dependencies()) == 2
    assert any(step.fact().source_path == "universe.lock.toml" for step in lock_page.dependencies())
    first = prior.direct_dependencies(lock, 1)
    cursor = first.continuation
    assert isinstance(cursor, pale.PortableWorkspaceDependencyCursor)
    next_bytes = prior.direct_dependencies(lock, 1, cursor).canonical_bytes
    for owner, subject, limit in ((fresh, lock, 1), (prior, leaf, 1), (prior, lock, 2)):
        try:
            owner.direct_dependencies(subject, limit, cursor)
        except pale.PaleDiagnosticError as error:
            wire = json.loads(error.diagnostic_bytes)
            assert wire["code"] == error.diagnostic.code == "workspace_dependency_continuation_mismatch"
            assert wire["effect"] == "not_applicable" and wire["summary"] == str(error)
        else:
            raise AssertionError("mismatched dependency cursor was accepted")
    inputless = prior.direct_dependencies(leaf, 1)
    assert inputless.root() is not None and inputless.dependencies() == [] and inputless.is_complete
    missing = prior.direct_dependencies(absent_subject, 1)
    assert missing.root() is None and missing.dependencies() == [] and missing.is_complete
    union_cursor = reference.subjects(1).continuation
    for bad_cursor in ({}, union_cursor, first):
        _reject(lambda: prior.direct_dependencies(lock, 1, bad_cursor), TypeError)
    _reject(lambda: reference.subjects(1, cursor), TypeError)
    _reject(lambda: prior.direct_dependencies({}, 1), TypeError)
    _reject(lambda: pale.PortableWorkspaceCompilation.direct_dependencies(reference, lock, 1), TypeError)
    for limit in (-1, 65536, 2**64):
        for after in (None, cursor):
            _reject(lambda: prior.direct_dependencies(lock, limit, after), OverflowError)
    class IndexLike:
        def __index__(self):
            raise AssertionError("page limits must not invoke __index__")

    for limit in (True, False, IndexLike(), 0.5, float("nan"), float("inf"), "1", {}, None):
        for after in (None, cursor):
            _reject(lambda: prior.direct_dependencies(lock, limit, after), TypeError)
    for cls in (pale.PortableWorkspaceDependencyPage, pale.PortableWorkspaceDependencyCursor):
        _reject(lambda: cls(), TypeError)
        _reject(lambda: cls.__new__(cls), TypeError)
    _reject(lambda: setattr(first, "is_complete", True), AttributeError)
    copied = first.dependencies()
    copied.clear()
    assert len(first.dependencies()) == 1
    assert prior.direct_dependencies(lock, 1, cursor).canonical_bytes == next_bytes
    assert terminal is not None
    before = terminal.canonical_bytes
    del reference, subjects_page, subjects
    gc.collect()
    assert terminal.canonical_bytes == before
    return {"direct_dependencies": True}


def _reject(call, expected):
    try:
        call()
    except expected:
        return
    raise AssertionError(f"expected {expected}")
