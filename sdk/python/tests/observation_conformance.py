"""Prove the shared bounded observation journey through PyPale."""

import json


def _generation(pale, value):
    assert value.kind == pale.ObservationGenerationKind.EffectiveProfile
    return {
        "kind": "effective-profile",
        "profileDigest": value.profile_digest,
        "instanceDigest": value.instance_digest,
        "compositionPlanDigest": value.composition_plan_digest,
    }


def _cursor(pale, value):
    assert value.revision.kind == pale.ObservationRevisionKind.Static
    return {
        "generation": _generation(pale, value.generation),
        "revision": {"kind": "static"},
    }


def _subject(pale, value):
    if value.kind == pale.ObservationSubjectKind.EffectiveProfile:
        return {"kind": "effective-profile", "profileDigest": value.profile_digest}
    assert value.kind == pale.ObservationSubjectKind.ExecutionPath
    return {"kind": "execution-path", "executionPathId": value.execution_path_id}


def _freshness(pale, value):
    if value.kind == pale.ObservationFreshnessKind.Fresh:
        return {"kind": "fresh"}
    assert value.kind == pale.ObservationFreshnessKind.Unknown
    return {"kind": "unknown"}


def _fact(pale, value):
    assert value.category == pale.ObservationCategory.Readiness
    if value.readiness_state == pale.ObservationReadinessState.Ready:
        state = "ready"
    else:
        assert value.readiness_state == pale.ObservationReadinessState.Unproven
        state = "unproven"
    return {
        "category": "readiness",
        "subject": _subject(pale, value.subject),
        "state": state,
        "freshness": _freshness(pale, value.freshness),
    }


def _completion(pale, value):
    if value.completion == pale.ObservationStepCompletionKind.Complete:
        kind = "complete"
    elif value.completion == pale.ObservationStepCompletionKind.Stopped:
        kind = "stopped"
    else:
        assert value.completion == pale.ObservationStepCompletionKind.Incomplete
        kind = "incomplete"
    if kind == "complete":
        return {"kind": kind}
    assert value.stopped_by == pale.ObservationStepLimitKind.Returned
    return {"kind": kind, "limit": {"kind": "returned"}}


def _causal_step(pale, value):
    variants = [
        (pale.ObservationCausalStepKind.OriginReceipt, "origin-receipt", "contractId"),
        (pale.ObservationCausalStepKind.EntryContract, "entry-contract", "contractId"),
        (pale.ObservationCausalStepKind.ResponsibilityContract,
            "responsibility-contract", "contractId",
        ),
        (pale.ObservationCausalStepKind.ProviderBinding, "provider-binding", "bindingId"),
        (pale.ObservationCausalStepKind.DelegationEdge, "delegation-edge", "edgeId"),
        (pale.ObservationCausalStepKind.TerminationContract,
            "termination-contract", "contractId",
        ),
        (pale.ObservationCausalStepKind.SettlementContract,
            "settlement-contract", "contractId",
        ),
        (pale.ObservationCausalStepKind.ContinuityContract,
            "continuity-contract", "contractId",
        ),
    ]
    kind, identity_key = next(
        (kind, identity_key)
        for variant, kind, identity_key in variants
        if value.kind == variant
    )
    return {"kind": kind, identity_key: value.reference_id}


def _causal_path(pale, value):
    return {
        "generation": _generation(pale, value.generation),
        "subject": _subject(pale, value.subject),
        "steps": [_causal_step(pale, step) for step in value.steps],
    }


def _page(pale, result):
    assert result.kind == pale.ObservationPageResultKind.Ready
    page = result.page
    continuation = None
    if page.continuation is not None:
        continuation = {
            "snapshot": _cursor(pale, page.continuation.snapshot),
            "inspectedPosition": page.continuation.inspected_position,
        }
    return {
        "kind": "ready",
        "page": {
            "cursor": _cursor(pale, page.cursor),
            "facts": [_fact(pale, fact) for fact in page.facts],
            "continuation": continuation,
            "completion": _completion(pale, page),
            "inspected": page.inspected,
            "copiedStringBytes": page.copied_string_bytes,
            "snapshotOmittedCount": page.snapshot_omitted_count,
        },
    }


def _lens(pale, result):
    assert result.kind == pale.ObservationLensDispositionKind.Ready
    lens = result.lens
    continuation = None
    if lens.continuation is not None:
        continuation = {
            "snapshot": _cursor(pale, lens.continuation.snapshot),
            "inspectedPosition": lens.continuation.inspected_position,
        }
    return {
        "kind": "ready",
        "lens": {
            "cursor": _cursor(pale, lens.cursor),
            "nodes": [
                {
                    "subject": _subject(pale, node.subject),
                    "facts": [_fact(pale, fact) for fact in node.facts],
                }
                for node in lens.nodes
            ],
            "causalPaths": [_causal_path(pale, path) for path in lens.causal_paths],
            "continuation": continuation,
            "completion": _completion(pale, lens),
            "inspectedSubjects": lens.inspected_subjects,
            "copiedStringBytes": lens.copied_string_bytes,
            "causalPathLookups": lens.causal_path_lookups,
            "causalSteps": lens.causal_steps,
            "snapshotOmittedCount": lens.snapshot_omitted_count,
        },
    }


def exercise_observation_conformance(pale, snapshot, fixture_root):
    expected = json.loads(
        (fixture_root / "observation-journey.json").read_text(encoding="utf-8")
    )
    page_limits = {
        "maximumInspected": 16,
        "maximumReturned": 1,
        "maximumCopiedStringBytes": 4096,
    }
    page_filter = pale.ObservationFilter()
    page_filter.select_category(pale.ObservationCategory.Readiness)
    page = snapshot.page(
        page_filter, pale.ObservationPageLimits(16, 1, 4096),
        pale.ObservationCancellation.Active, None,
    )
    mismatch = snapshot.page(
        page_filter, pale.ObservationPageLimits(16, 2, 4096),
        pale.ObservationCancellation.Active, page.page.continuation,
    )
    assert mismatch.kind == pale.ObservationPageResultKind.Resynchronize
    assert (
        mismatch.resynchronization.reason
        == pale.ObservationResynchronizationReason.ContinuationMismatch
    )

    discovery = snapshot.page(
        pale.ObservationFilter(), pale.ObservationPageLimits(256, 256, 2**32 - 1),
        pale.ObservationCancellation.Active, None,
    )
    execution_path = next(
        fact.subject for fact in discovery.page.facts
        if fact.subject is not None
        and fact.subject.kind == pale.ObservationSubjectKind.ExecutionPath
    )
    causal = snapshot.causal_path(execution_path)
    assert causal.kind == pale.ObservationCausalResultKind.Found

    lens_filter = pale.ObservationFilter()
    lens_filter.select_subject(execution_path)
    lens_limits = {
        "maximumInspected": 256,
        "maximumNodes": 1,
        "maximumCopiedStringBytes": 4096,
        "maximumCausalPathLookups": 1,
        "maximumCausalSteps": 256,
    }
    lens = snapshot.visual_lens(
        lens_filter, pale.ObservationLensLimits(256, 1, 4096, 1, 256), True,
        pale.ObservationCancellation.Active, None,
    )
    actual = {
        "snapshot": {
            "cursor": _cursor(pale, snapshot.cursor),
            "factCount": snapshot.fact_count,
            "omittedFactCount": snapshot.omitted_fact_count,
        },
        "pageLimits": page_limits,
        "page": _page(pale, page),
        "continuationMismatch": {
            "kind": "resynchronize",
            "resynchronization": {
                "reason": {"kind": "continuation-mismatch"},
                "current": _cursor(pale, mismatch.resynchronization.current),
            },
        },
        "causalPath": {"kind": "ready", "path": _causal_path(pale, causal.path)},
        "lensLimits": lens_limits,
        "lens": _lens(pale, lens),
    }
    assert actual == expected
