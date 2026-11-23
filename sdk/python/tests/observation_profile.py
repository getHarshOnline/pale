"""Read original profile rows through real opaque observation handles."""

import pypale as pale
from effective_profile_entries import _assert_entry


def exercise_observation_profile(api, workspace, profile, rebuilt_profile, foreign_workspace, changed_profile, rows):
    snapshot = api.observe_effective_profile(profile)
    facts = snapshot.page(
        pale.ObservationFilter(),
        pale.ObservationPageLimits(256, 256, 2**32 - 1),
        pale.ObservationCancellation.Active, None,
    ).page
    assert facts is not None and facts.continuation is None
    subject = next(fact.subject for fact in facts.facts if fact.subject is not None and fact.subject.kind == pale.ObservationSubjectKind.Machine)

    def ready(result):
        assert result.kind == pale.ObservationProfileResultKind.Ready
        assert result.error is None and result.page is not None
        return result.page

    def refuse(result, error):
        assert result.kind == pale.ObservationProfileResultKind.Refused
        assert result.error == error and result.page is None

    full = ready(snapshot.inspect_profile(profile, workspace, subject, 256, 256))
    assert full.mapping == pale.ObservationProfileMapping.PrimaryRows
    assert full.entries and full.continuation is None
    assert full.coordinates.instance_digest == profile.instance_digest
    assert full.coordinates.profile_digest == profile.profile_digest
    for entry in full.entries:
        _assert_entry(entry, rows[entry.explanation_handle])
        assert profile.explain(entry.explanation_handle).handle == entry.explanation_handle

    first = ready(snapshot.inspect_profile(profile, workspace, subject, 1, 1))
    assert first.continuation is not None
    handles = [entry.explanation_handle for entry in first.entries]
    inspected = first.inspected
    page = first
    calls = 1
    while page.continuation is not None:
        assert calls <= full.inspected + 1
        calls += 1
        page = ready(snapshot.inspect_profile(profile, workspace, subject, 1, 1, page.continuation))
        assert page.inspected <= 1 and len(page.entries) <= 1
        inspected += page.inspected
        handles.extend(entry.explanation_handle for entry in page.entries)
    assert inspected == full.inspected
    assert handles == [entry.explanation_handle for entry in full.entries]
    rebuilt = ready(snapshot.inspect_profile(rebuilt_profile, workspace, subject, 256, 256))
    assert [entry.explanation_handle for entry in rebuilt.entries] == handles

    error = pale.ObservationProfileError
    for visited, returned in ((0, 1), (1, 0), (257, 1), (1, 257)):
        refuse(snapshot.inspect_profile(profile, workspace, subject, visited, returned), error.InvalidLimit)
    refuse(snapshot.inspect_profile(profile, foreign_workspace, subject, 1, 1), error.SourceMismatch)
    refuse(snapshot.inspect_profile(changed_profile, workspace, subject, 1, 1), error.CoordinateMismatch)
    refuse(snapshot.inspect_profile(profile, workspace, subject, 2, 1, first.continuation), error.ContinuationMismatch)
    second_snapshot = api.observe_effective_profile(profile)
    refuse(second_snapshot.inspect_profile(profile, workspace, subject, 1, 1, first.continuation), error.ContinuationMismatch)
    whole = next(fact.subject for fact in facts.facts if fact.subject is not None and fact.subject.kind == pale.ObservationSubjectKind.EffectiveProfile)
    unmapped = ready(snapshot.inspect_profile(profile, workspace, whole, 256, 256))
    assert unmapped.mapping == pale.ObservationProfileMapping.NoPrimaryRows and not unmapped.entries
    refuse(snapshot.inspect_profile(profile, workspace, whole, 1, 1, first.continuation), error.ContinuationMismatch)
    return "observation.profile-inspection"
