"""Selected-identity union laws through the installed public wheel."""

import gc
import json

import pypale as pale


def exercise_comparison_subjects(api, prior, selected, added_subject, fresh):
    forward = api.compare_portable_workspaces(prior, selected)
    reverse = api.compare_portable_workspaces(selected, prior)
    repeated = api.compare_portable_workspaces(prior, selected)
    reference = api.compare_portable_workspaces(prior, prior)

    def wire_subject(subject):
        return json.loads(reference.compare_subject(subject).canonical_bytes)["subject"]

    def key(subject):
        return json.dumps(wire_subject(subject), sort_keys=True)

    def selected_subjects(owner):
        page = api.compare_portable_workspaces(owner, owner).impacts(0, 256)
        assert page.next_offset is None, "the existing small fixture fits this reference"
        return [entry.subject() for entry in page.entries]

    sides = [list(map(key, selected_subjects(owner))) for owner in (prior, selected)]
    expected = set(sides[0]) | set(sides[1])
    order = None
    terminal = None
    for owner in (forward, reverse):
        for limit in (1, 2, 0, 256, 65535):
            cursor = None
            actual = []
            for _ in range(256):
                page = owner.subjects(limit, cursor)
                assert isinstance(page, pale.WorkspaceComparisonSubjectPage)
                assert page.belongs_to(owner)
                wire = json.loads(page.canonical_bytes)
                summary = page.comparison()
                summary_wire = json.loads(summary.canonical_bytes)
                for field in ("prior_principal_universe_digest", "current_principal_universe_digest",
                              "prior_semantic_ir_digest", "current_semantic_ir_digest", "receipt_digest",
                              "change_root_count", "affected_count", "unaffected_count"):
                    assert getattr(summary, field) == getattr(owner, field) == summary_wire[field]
                for field, name in (("prior_principal_universe_digest", "priorPrincipalUniverseDigest"),
                                    ("current_principal_universe_digest", "currentPrincipalUniverseDigest"),
                                    ("prior_semantic_ir_digest", "priorSemanticIrDigest"),
                                    ("current_semantic_ir_digest", "currentSemanticIrDigest"),
                                    ("receipt_digest", "receiptDigest")):
                    assert wire[name] == getattr(summary, field)
                rows = page.subjects()
                assert [wire_subject(subject) for subject in rows] == wire["subjects"]
                assert len(rows) <= min(limit or 64, 256)
                assert all(not isinstance(owner.compare_subject(subject).values,
                                          pale.AbsentWorkspaceSubjectValues) for subject in rows)
                assert page.inspected == wire["inspected"]
                assert len(rows) <= page.inspected <= 2 * len(rows) + 2
                assert page.key_comparisons == wire["keyComparisons"]
                assert 0 <= page.key_comparisons <= len(rows)
                assert page.is_complete == wire["isComplete"] == (page.continuation is None)
                actual.extend(map(key, rows))
                cursor = page.continuation
                if cursor is None:
                    terminal = page
                    break
            else:
                raise AssertionError("subject enumeration did not terminate")
            assert len(actual) == len(set(actual)) and set(actual) == expected
            if order is None:
                order = actual
            assert actual == order
            for side in sides:
                assert [subject for subject in actual if subject in set(side)] == side

    assert isinstance(reverse.compare_subject(added_subject).values, pale.PriorOnlyWorkspaceSubjectValues)
    assert key(added_subject) in expected
    assert key(added_subject) not in {key(entry.subject()) for entry in reverse.impacts(0, 256).entries}
    first = forward.subjects(1)
    cursor = first.continuation
    assert isinstance(cursor, pale.WorkspaceComparisonSubjectCursor)
    next_bytes = forward.subjects(1, cursor).canonical_bytes
    assert first.belongs_to(repeated)
    assert repeated.subjects(1, cursor).canonical_bytes == next_bytes
    distinct_pair = api.compare_portable_workspaces(fresh, selected)
    assert distinct_pair.prior_principal_universe_digest == forward.prior_principal_universe_digest
    for owner, limit in ((reverse, 1), (distinct_pair, 1), (forward, 2)):
        try:
            owner.subjects(limit, cursor)
        except pale.PaleDiagnosticError as error:
            wire = json.loads(error.diagnostic_bytes)
            assert error.diagnostic.code == wire["code"] == "workspace_comparison_subject_continuation_mismatch"
            assert wire["effect"] == "not_applicable" and wire["summary"] == str(error)
        else:
            raise AssertionError("mismatched subject cursor was accepted")
    assert not first.belongs_to(reverse) and not first.belongs_to(distinct_pair)
    for invalid in (-1, 65536, 2**64):
        for after in (None, cursor):
            _reject(lambda: forward.subjects(invalid, after), OverflowError)
    class IndexLike:
        def __index__(self):
            raise AssertionError("page limits must not invoke __index__")

    for invalid in (True, False, IndexLike(), 0.5, float("nan"), float("inf"), "1", {}, None):
        for after in (None, cursor):
            _reject(lambda: forward.subjects(invalid, after), TypeError)
    for invalid in ({}, prior, first):
        _reject(lambda: forward.subjects(1, invalid), TypeError)
    _reject(lambda: pale.WorkspaceComparison.subjects(prior, 1), TypeError)
    for cls in (pale.WorkspaceComparisonSubjectPage, pale.WorkspaceComparisonSubjectCursor,
                pale.WorkspaceComparisonSummary):
        _reject(lambda: cls(), TypeError)
        _reject(lambda: cls.__new__(cls), TypeError)
    _reject(lambda: setattr(first, "inspected", 0), AttributeError)
    rows = first.subjects()
    rows.clear()
    assert len(first.subjects()) == 1
    changed = bytearray(first.canonical_bytes)
    changed[0] ^= 1
    assert json.loads(first.canonical_bytes)["subjects"]
    assert forward.subjects(1, cursor).canonical_bytes == next_bytes
    assert terminal is not None and terminal.is_complete
    terminal_bytes = terminal.canonical_bytes
    del forward, reverse, repeated, reference, distinct_pair
    gc.collect()
    assert terminal.canonical_bytes == terminal_bytes
    assert terminal.comparison().receipt_digest
    return {"comparison_subjects": True}


def _reject(call, expected):
    try:
        call()
    except expected:
        return
    raise AssertionError(f"expected {expected}")
