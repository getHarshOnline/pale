"""Exercise retained authoring queries through the installed native Python ABI."""

import gc
import json

import pypale as pale

from effective_profile_entries import _assert_entry


def _location(location):
    return location.source_label, tuple(
        (part.kind, part.field_value if part.kind == "field" else part.index_value)
        for part in location.path
    )


def _occurrences(compilation, identity, inspected=1, returned=1):
    cursor = None
    occurrences = []
    empty_incomplete = False
    for _ in range(10_000):
        page = compilation.authoring_occurrences(identity, inspected, returned, cursor)
        assert page.inspected <= (64 if inspected == 0 else min(inspected, 256))
        assert len(page.occurrences) <= (64 if returned == 0 else min(returned, 256))
        expected_bytes = sum(
            len(item.location.source_label.encode())
            + sum(len(part.field_value.encode()) for part in item.location.path if part.kind == "field")
            for item in page.occurrences
        )
        assert page.copied_string_bytes == expected_bytes
        occurrences.extend((_location(item.location), item.role) for item in page.occurrences)
        cursor = page.continuation
        assert page.is_complete == (cursor is None)
        empty_incomplete |= not page.occurrences and not page.is_complete
        if page.is_complete:
            return occurrences, empty_incomplete
    raise AssertionError("occurrence continuation made no bounded progress")


def _rejected(call, exception):
    try:
        call()
    except exception as error:
        return error
    raise AssertionError(f"expected {exception.__name__}")


def _cursor_rejected(call):
    error = _rejected(call, pale.PaleDiagnosticError)
    diagnostic = json.loads(error.diagnostic_bytes)
    assert error.diagnostic.code == diagnostic["code"] == "authoring_continuation_mismatch"
    assert str(error) == diagnostic["summary"]
    assert diagnostic["effect"] == "not_applicable"
    return error.diagnostic_bytes


def _strict_limits(call):
    class IndexOnly:
        def __index__(self):
            raise AssertionError("page limit must not invoke an arbitrary index conversion")

    for value in (True, False, 1.0, 1.5, float("inf"), float("nan"), "1", None, IndexOnly()):
        _rejected(lambda: call(value), TypeError)
    for value in (-1, 65_536, 1 << 100):
        _rejected(lambda: call(value), OverflowError)


def exercise_authoring_pages(api, fixture_root, portable, documents, prior, current, effective):
    for name in (
        "AuthoringOccurrence", "AuthoringOccurrencePage", "AuthoringOccurrencePageCursor",
        "EffectiveAuthoringPage", "EffectiveAuthoringContinuation",
    ):
        cls = getattr(pale, name)
        _rejected(lambda: cls(), TypeError)
        _rejected(lambda: cls.__new__(cls), TypeError)

    path = [pale.AuthoringInstancePathSegment.field("workspace"),
            pale.AuthoringInstancePathSegment.field("workspace_id")]
    navigation = prior.navigate_authoring(portable["root_manifest"], path)
    identity = navigation.identity
    assert identity is not None and navigation.definition is not None
    assert not navigation.references_truncated
    expected = []
    for location in [navigation.definition, *navigation.references]:
        resolved = prior.navigate_authoring(location.source_label, location.path)
        assert resolved.identity is not None
        assert (resolved.identity.kind.name, resolved.identity.value, resolved.identity.authored_value) == (
            identity.kind.name, identity.value, identity.authored_value,
        )
        expected.append((_location(location), resolved.occurrence_role))
    found, _ = _occurrences(prior, identity)
    assert sorted(found) == sorted(expected)
    assert len(found) == len({location for location, _ in found})
    for inspected, returned in ((0, 0), (256, 256), (65_535, 65_535), (3, 1), (1, 3)):
        assert _occurrences(prior, identity, inspected, returned)[0] == found
    first = prior.authoring_occurrences(identity, 1, 1)
    cursor = first.continuation
    assert cursor is not None
    _cursor_rejected(lambda: current.authoring_occurrences(identity, 1, 1, cursor))
    _cursor_rejected(lambda: prior.authoring_occurrences(identity, 2, 1, cursor))
    _cursor_rejected(lambda: prior.authoring_occurrences(identity, 1, 2, cursor))
    default_cursor = prior.authoring_occurrences(identity, 0, 1).continuation
    assert default_cursor is not None
    _cursor_rejected(lambda: prior.authoring_occurrences(identity, 64, 1, default_cursor))
    _strict_limits(lambda value: prior.authoring_occurrences(identity, value, 1))
    _strict_limits(lambda value: prior.authoring_occurrences(identity, 1, value))

    effective_identity = effective.navigate_authoring("pale-workspace.toml", path).identity
    assert effective_identity is not None
    _cursor_rejected(lambda: prior.authoring_occurrences(effective_identity, 1, 1, cursor))
    _cursor_rejected(lambda: effective.authoring_occurrences(identity, 1, 1, cursor))
    assert _occurrences(prior, effective_identity)[0] == []
    effective_found, empty_incomplete = _occurrences(effective, effective_identity)
    assert effective_found and empty_incomplete
    effective_cursor = effective.authoring_occurrences(effective_identity, 1, 1).continuation
    assert effective_cursor is not None
    _cursor_rejected(lambda: prior.authoring_occurrences(effective_identity, 1, 1, effective_cursor))
    _strict_limits(lambda value: effective.authoring_occurrences(effective_identity, value, 1))
    _strict_limits(lambda value: effective.authoring_occurrences(effective_identity, 1, value))

    repository_path = [pale.AuthoringInstancePathSegment.field("workspace"),
                       pale.AuthoringInstancePathSegment.field("desired_topology"),
                       pale.AuthoringInstancePathSegment.field("repositories"),
                       pale.AuthoringInstancePathSegment.index(0),
                       pale.AuthoringInstancePathSegment.field("id")]
    repository = effective.navigate_authoring("pale-workspace.toml", repository_path).identity
    assert repository is not None
    legacy = effective.inspect_effective_authoring(repository)
    first_profile = effective.inspect_effective_authoring_page(repository, 1)
    profile_cursor = first_profile.continuation
    assert profile_cursor is not None, "fixture must exercise a real association continuation"
    _cursor_rejected(lambda: effective.inspect_effective_authoring_page(repository, 2, profile_cursor))
    _cursor_rejected(lambda: effective.inspect_effective_authoring_page(effective_identity, 1, profile_cursor))
    _strict_limits(lambda value: effective.inspect_effective_authoring_page(repository, value))
    absent = effective.inspect_effective_authoring_page(effective_identity, 1)
    assert not absent.entries and absent.continuation is None

    canonical_rows = {}
    for section in (
        pale.EffectiveProfileSection.IntendedUseCase, pale.EffectiveProfileSection.EffectiveConfiguration,
        pale.EffectiveProfileSection.CapabilitiesAndOperations, pale.EffectiveProfileSection.MachineRealization,
        pale.EffectiveProfileSection.GapsAndExplanations,
    ):
        section_page = effective.inspect(section, 0, 256)
        assert section_page.next_offset is None
        for row in json.loads(section_page.canonical_bytes)["entries"]:
            canonical_rows[row["explanationHandle"]] = row
    found_handles = []
    page = first_profile
    for _ in range(100):
        assert page.coordinates.instance_digest == effective.instance_digest
        assert page.coordinates.selected_topology_digest == legacy.coordinates.selected_topology_digest
        for row in page.entries:
            _assert_entry(row, canonical_rows[row.explanation_handle])
            assert effective.explain(row.explanation_handle).handle == row.explanation_handle
            found_handles.append(row.explanation_handle)
        if page.continuation is None:
            break
        page = effective.inspect_effective_authoring_page(repository, 1, page.continuation)
    else:
        raise AssertionError("profile continuation made no bounded progress")
    assert found_handles == [row.explanation_handle for row in legacy.entries]
    assert len(found_handles) == len(set(found_handles)) and not legacy.truncated
    for limit in (0, 64, 256, 65_535):
        complete = effective.inspect_effective_authoring_page(repository, limit)
        assert complete.continuation is None
        assert [row.explanation_handle for row in complete.entries] == found_handles

    # A separately compiled profile with equal coordinates remains a foreign owner.
    one_shot = json.loads((fixture_root / "one-shot-journey.json").read_text())
    source = fixture_root / "one-shot"
    document = pale.CompilationDocument
    workspace = api.compile_portable_workspace("pale-workspace.toml", [
        (name, (source / "workspace" / name).read_bytes())
        for name in ("composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml")
    ])
    fresh = api.compile_effective_profile(workspace, pale.EffectiveProfileCompilationRequest(
        project_id=one_shot["selectors"]["project_id"],
        execution_workspace_id=one_shot["selectors"]["execution_workspace_id"],
        machine_id=one_shot["selectors"]["machine_id"],
        private_inputs=pale.OneShotPrivateInputs(
            capabilities=document("capabilities.toml", (source / "private/capabilities.toml").read_bytes()),
            machine_binding=document("machines/machine.toml", (source / "private/machine-binding.toml").read_bytes()),
            distribution_lock=document("distribution-lock.toml", (source / "private/distribution-lock.toml").read_bytes()),
            lifecycle_guidance=[],
        ),
        package_documents=[document(f"engine/{name}", (source / "packages/engine" / name).read_bytes())
                           for name in ("package.toml", "role.md", "roles.toml")],
    ))
    assert fresh.instance_digest == effective.instance_digest
    _cursor_rejected(lambda: fresh.inspect_effective_authoring_page(repository, 1, profile_cursor))
    _cursor_rejected(lambda: fresh.authoring_occurrences(effective_identity, 1, 1, effective_cursor))
    retained_page = fresh.inspect_effective_authoring_page(repository, 1)
    retained_cursor = retained_page.continuation
    retained_occurrence_page = fresh.authoring_occurrences(effective_identity, 65_535, 65_535)
    retained_occurrences = retained_occurrence_page.occurrences
    assert retained_occurrences
    del fresh, workspace
    gc.collect()
    assert retained_page.entries[0].explanation_handle == found_handles[0]
    assert retained_cursor is not None and retained_occurrences[0].location.source_label
    _cursor_rejected(lambda: effective.inspect_effective_authoring_page(repository, 1, retained_cursor))
    retained = api.compile_portable_workspace(portable["root_manifest"], documents)
    alias = retained
    retained_cursor = retained.authoring_occurrences(identity, 1, 1).continuation
    assert retained_cursor is not None
    retained_result = retained.authoring_occurrences(identity, 65_535, 65_535)
    del retained
    gc.collect()
    alias.authoring_occurrences(identity, 1, 1, retained_cursor)
    del alias
    gc.collect()
    assert [(_location(item.location), item.role) for item in retained_result.occurrences] == found
