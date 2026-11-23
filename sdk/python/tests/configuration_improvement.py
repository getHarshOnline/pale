"""Exercise both configuration-improvement routes through the installed ABI."""

import json
import gc
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from tempfile import TemporaryDirectory
from threading import Barrier

import pypale


def exercise_local_configuration_improvement(root: Path) -> None:
    """Use real local sources, no candidate Package, and existing review handles."""
    candidate = root / "candidate"
    candidate.mkdir()
    for name in ["composition.toml", "pale-workspace.toml",
                 "workspace-resolution.lock.toml", "workspace-universe.lock.toml",
                 "capabilities.toml", "distribution-lock.toml", "machines/machine.toml"]:
        destination = candidate / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        source = (root / name).read_text(encoding="utf-8")
        if name == "pale-workspace.toml":
            source = source.replace('access = "mutate"', 'access = "read"')
        if name == "machines/machine.toml":
            source = source.replace(str(root / "distribution"), str(candidate / "absent-package"))
        destination.write_text(source, encoding="utf-8")
    assert not (candidate / "absent-package").exists()
    before = {str(path.relative_to(root)): None if path.is_dir() else path.read_bytes()
              for path in root.rglob("*")}
    request = pypale.LocalConfigurationImprovementRequest(
        baseline_manifest=str(root / "pale-workspace.toml"),
        candidate_manifest=str(candidate / "pale-workspace.toml"),
        project_id="demo", execution_workspace_id="default", machine_id="machine",
    )
    for method, disposition in [
        ("accept", pypale.ConfigurationImprovementDisposition.Accepted),
        ("reject", pypale.ConfigurationImprovementDisposition.Rejected),
        ("cancel", pypale.ConfigurationImprovementDisposition.Cancelled),
        ("mark_stale", pypale.ConfigurationImprovementDisposition.Stale),
    ]:
        api = pypale.Pale()
        preview = api.preview_local_configuration_improvement(request)
        del api
        gc.collect()
        assert preview.phase == pypale.ConfigurationImprovementPhase.Previewed
        summary = preview.summary
        baseline_profile = preview.baseline_profile()
        candidate_profile = preview.candidate_profile()
        assert baseline_profile.profile_digest == summary.profile_digest
        assert candidate_profile.profile_digest == summary.candidate_profile_digest
        assert summary.profile_digest != summary.candidate_profile_digest
        machine_rows = json.loads(candidate_profile.inspect(
            pypale.EffectiveProfileSection.MachineRealization, 0, 1,
        ).canonical_bytes)["entries"]
        profile_explanation = candidate_profile.explain(machine_rows[0]["explanationHandle"])
        assert profile_explanation is not None
        assert isinstance(profile_explanation.evidence, pypale.EffectiveProfileMachineRealizationEvidence)
        assert profile_explanation.evidence.realization == pypale.EffectiveProfileNativeRealization.Unknown
        comparison = preview.comparison()
        assert comparison.change_root_count > 0
        comparison.impacts(0, 1)
        terminal = getattr(preview, method)()
        assert terminal.disposition == disposition
        assert terminal.preview.canonical_bytes == summary.canonical_bytes
        try:
            preview.accept()
        except ValueError:
            pass
        else:
            raise AssertionError("local preview was consumed twice")
        assert baseline_profile.workspace_id == "act-ws"
    api = pypale.Pale()
    bad = pypale.LocalConfigurationImprovementRequest(
        baseline_manifest="relative.toml", candidate_manifest=str(candidate / "pale-workspace.toml"),
        project_id="demo", execution_workspace_id="default", machine_id="machine",
    )
    try:
        api.preview_local_configuration_improvement(bad)
    except pypale.PaleDiagnosticError as error:
        preview_error = error.diagnostic_bytes
        assert b"relative.toml" not in preview_error
    else:
        raise AssertionError("relative locator accepted")
    try:
        api.compile_local_one_shot(pypale.LocalOneShotCompilationRequest(
            workspace_manifest="relative.toml", project_id="demo",
            execution_workspace_id="default", machine_id="machine"))
    except pypale.PaleDiagnosticError as error:
        assert error.diagnostic_bytes == preview_error
    else:
        raise AssertionError("relative compilation locator accepted")
    after = {str(path.relative_to(root)): None if path.is_dir() else path.read_bytes()
             for path in root.rglob("*")}
    assert before == after
    assert not (candidate / "absent-package").exists()


def _document(label: str, path: Path) -> pypale.CompilationDocument:
    return pypale.CompilationDocument(label, path.read_bytes())


def _workspace_documents(root: Path, access: str) -> list[pypale.CompilationDocument]:
    manifest = (root / "workspace/pale-workspace.toml").read_text(encoding="utf-8")
    manifest = manifest.replace('access = "mutate"', f'access = "{access}"')
    return [
        _document("composition.toml", root / "workspace/composition.toml"),
        pypale.CompilationDocument("pale-workspace.toml", manifest.encode()),
        _document(
            "workspace-resolution.lock.toml",
            root / "workspace/workspace-resolution.lock.toml",
        ),
        _document(
            "workspace-universe.lock.toml",
            root / "workspace/workspace-universe.lock.toml",
        ),
    ]


def _portable_documents(root: Path, access: str) -> list[tuple[str, bytes]]:
    manifest = (root / "workspace/pale-workspace.toml").read_text(encoding="utf-8")
    manifest = manifest.replace('access = "mutate"', f'access = "{access}"')
    return [
        ("composition.toml", (root / "workspace/composition.toml").read_bytes()),
        ("pale-workspace.toml", manifest.encode()),
        (
            "workspace-resolution.lock.toml",
            (root / "workspace/workspace-resolution.lock.toml").read_bytes(),
        ),
        (
            "workspace-universe.lock.toml",
            (root / "workspace/workspace-universe.lock.toml").read_bytes(),
        ),
    ]


def _private_inputs(
    api: pypale.Pale,
    root: Path,
    machine_root: str,
) -> pypale.OneShotPrivateInputs:
    identity = api.contract_bundle_identity()
    lock = (root / "private/distribution-lock.toml").read_text(encoding="utf-8")
    lines = lock.splitlines()
    lines[2] = f'schema_manifest_sha256 = "{identity.schema_manifest_sha256}"'
    lines[3] = f'schema_set_sha256 = "{identity.schema_set_sha256}"'
    machine = (root / "private/machine-binding.toml").read_text(encoding="utf-8")
    machine = machine.replace("/captured/repository", machine_root)
    return pypale.OneShotPrivateInputs(
        capabilities=_document("capabilities.toml", root / "private/capabilities.toml"),
        machine_binding=pypale.CompilationDocument(
            "machines/machine.toml", machine.encode()
        ),
        distribution_lock=pypale.CompilationDocument(
            "distribution-lock.toml", ("\n".join(lines) + "\n").encode()
        ),
        lifecycle_guidance=[],
    )


def _invalid_private_inputs() -> pypale.OneShotPrivateInputs:
    invalid = b"invalid = ["
    return pypale.OneShotPrivateInputs(
        capabilities=pypale.CompilationDocument("capabilities.toml", invalid),
        machine_binding=pypale.CompilationDocument("machines/machine.toml", invalid),
        distribution_lock=pypale.CompilationDocument("distribution-lock.toml", invalid),
        lifecycle_guidance=[],
    )


def _package_documents(root: Path) -> list[pypale.CompilationDocument]:
    return [
        _document(f"engine/{name}", root / "packages/engine" / name)
        for name in ["package.toml", "role.md", "roles.toml"]
    ]


def _explanation(
    api: pypale.Pale,
    baseline: pypale.PortableWorkspaceCompilation,
    profile: pypale.EffectiveProfileCompilation,
) -> pypale.ConfigurationImprovementExplanation:
    inspection = api.inspect_configuration_improvement(baseline)
    assert inspection.phase == pypale.ConfigurationImprovementPhase.Inspected
    assert inspection.workspace_id == "act-ws"
    explanation = api.explain_configuration_improvement(inspection, profile)
    assert explanation.phase == pypale.ConfigurationImprovementPhase.Explained
    try:
        api.explain_configuration_improvement(inspection, profile)
    except ValueError:
        pass
    else:
        raise AssertionError("an inspected handle must be consumed exactly once")
    return explanation


def _direct_preview(
    api: pypale.Pale,
    root: Path,
    baseline: pypale.PortableWorkspaceCompilation,
    profile: pypale.EffectiveProfileCompilation,
) -> pypale.ConfigurationImprovementPreview:
    explanation = _explanation(api, baseline, profile)
    proposal = api.propose_configuration_improvement(
        explanation,
        pypale.ConfigurationImprovementProposalRequest(
            project_id="demo",
            execution_workspace_id="default",
            machine_id="machine",
            workspace=pypale.OneShotWorkspaceInputs(
                "pale-workspace.toml", _workspace_documents(root, "read")
            ),
            private_inputs=_private_inputs(
                api, root, "/captured/repository-candidate"
            ),
        ),
    )
    try:
        explanation.phase
    except ValueError:
        pass
    else:
        raise AssertionError("proposal creation must consume its explanation")
    assert proposal.phase == pypale.ConfigurationImprovementPhase.Proposed
    evaluation = api.evaluate_configuration_improvement(proposal)
    try:
        proposal.phase
    except ValueError:
        pass
    else:
        raise AssertionError("evaluation must consume its proposal")
    assert evaluation.phase == pypale.ConfigurationImprovementPhase.Evaluated
    assert len(evaluation.candidate_principal_universe_digest) == 64
    assert len(evaluation.candidate_profile_digest) == 64
    assert len(evaluation.candidate_instance_digest) == 64
    diagnosis = api.diagnose_configuration_improvement(evaluation)
    try:
        evaluation.phase
    except ValueError:
        pass
    else:
        raise AssertionError("diagnosis must consume its evaluation")
    assert diagnosis.phase == pypale.ConfigurationImprovementPhase.Diagnosed
    assert len(diagnosis.derivation_receipt_digest) == 64
    preview = api.preview_configuration_improvement(diagnosis)
    try:
        diagnosis.phase
    except ValueError:
        pass
    else:
        raise AssertionError("preview creation must consume its diagnosis")
    assert preview.phase == pypale.ConfigurationImprovementPhase.Previewed
    return preview


def _prepared_preview(
    api: pypale.Pale,
    root: Path,
    baseline: pypale.PortableWorkspaceCompilation,
    candidate: pypale.PortableWorkspaceCompilation,
    profile: pypale.EffectiveProfileCompilation,
) -> pypale.ConfigurationImprovementPreview:
    explanation = _explanation(api, baseline, profile)
    prepared = api.prepare_configuration_improvement_preview(
        explanation,
        candidate,
        pypale.ConfigurationImprovementCandidateRequest(
            project_id="demo",
            execution_workspace_id="default",
            machine_id="machine",
            private_inputs=_private_inputs(
                api, root, "/captured/repository-candidate"
            ),
        ),
    )
    prepared_summary = prepared.summary
    preview = api.commit_configuration_improvement_preview(explanation, prepared)
    assert prepared_summary.canonical_bytes == preview.summary.canonical_bytes
    try:
        prepared.summary
    except ValueError:
        pass
    else:
        raise AssertionError("a committed preparation must reject fresh observations")
    return preview


def exercise_configuration_improvement(api: pypale.Pale, fixture_root: Path) -> None:
    """Prove the installed ABI is a linear, effect-free projection of the API owner."""

    root = fixture_root / "one-shot"
    golden = json.loads(
        (fixture_root / "improvement-journey.json").read_text(encoding="utf-8")
    )
    baseline = api.compile_portable_workspace(
        "pale-workspace.toml", _portable_documents(root, "mutate")
    )
    candidate = api.compile_portable_workspace(
        "pale-workspace.toml", _portable_documents(root, "read")
    )
    profile = api.compile_effective_profile(
        baseline,
        pypale.EffectiveProfileCompilationRequest(
            project_id="demo",
            execution_workspace_id="default",
            machine_id="machine",
            private_inputs=_private_inputs(api, root, "/captured/repository"),
            package_documents=_package_documents(root),
        ),
    )

    with TemporaryDirectory(prefix="pale-python-improvement-") as temporary:
        sentinel = Path(temporary) / "no-native-effect"
        sentinel.write_text("unchanged\n", encoding="utf-8")
        direct = _direct_preview(api, root, baseline, profile)
        prepared = _prepared_preview(api, root, baseline, candidate, profile)
        direct_summary = direct.summary
        assert json.loads(direct_summary.canonical_bytes) == golden["summary"]
        assert direct_summary.canonical_bytes == prepared.summary.canonical_bytes
        assert sentinel.read_text(encoding="utf-8") == "unchanged\n"

    comparison = direct.comparison()
    assert {
        "prior_principal_universe_digest": comparison.prior_principal_universe_digest,
        "current_principal_universe_digest": comparison.current_principal_universe_digest,
        "prior_semantic_ir_digest": comparison.prior_semantic_ir_digest,
        "current_semantic_ir_digest": comparison.current_semantic_ir_digest,
        "receipt_digest": comparison.receipt_digest,
        "change_root_count": comparison.change_root_count,
        "affected_count": comparison.affected_count,
        "unaffected_count": comparison.unaffected_count,
    } == {
        key: golden["comparison"][key]
        for key in [
            "prior_principal_universe_digest",
            "current_principal_universe_digest",
            "prior_semantic_ir_digest",
            "current_semantic_ir_digest",
            "receipt_digest",
            "change_root_count",
            "affected_count",
            "unaffected_count",
        ]
    }
    impacts = comparison.impacts(0, 1)
    subject = impacts.entries[0].subject()
    assert comparison.explain(subject).subject().kind == subject.kind
    baseline_profile = direct.baseline_profile()
    candidate_profile = direct.candidate_profile()
    assert baseline_profile.profile_digest == direct_summary.profile_digest
    assert candidate_profile.profile_digest == direct_summary.candidate_profile_digest
    navigation = candidate_profile.navigate_authoring(
        "pale-workspace.toml",
        [
            pypale.AuthoringInstancePathSegment.field("workspace"),
            pypale.AuthoringInstancePathSegment.field("workspace_id"),
        ],
    )
    assert navigation.identity.value == "act-ws"

    terminal = direct.accept()
    assert terminal.disposition == pypale.ConfigurationImprovementDisposition.Accepted
    assert json.loads(terminal.preview.canonical_bytes) == golden["summary"]
    try:
        direct.reject()
    except ValueError:
        pass
    else:
        raise AssertionError("a preview must have exactly one terminal disposition")
    for fresh_observation in (
        lambda: direct.phase,
        lambda: direct.summary,
        direct.comparison,
    ):
        try:
            fresh_observation()
        except ValueError:
            pass
        else:
            raise AssertionError(
                "a consumed preview must reject every fresh stage observation"
            )
    try:
        comparison.impacts(0, 1)
    except ValueError:
        pass
    else:
        raise AssertionError("a borrowed comparison must close with its preview")
    assert baseline_profile.workspace_id == "act-ws"

    owner = _explanation(api, baseline, profile)
    foreign = _explanation(api, baseline, profile)
    try:
        api.prepare_configuration_improvement_preview(
            owner,
            candidate,
            pypale.ConfigurationImprovementCandidateRequest(
                project_id="demo",
                execution_workspace_id="default",
                machine_id="machine",
                private_inputs=_invalid_private_inputs(),
            ),
        )
    except pypale.PaleDiagnosticError:
        pass
    else:
        raise AssertionError("invalid private input must fail before consuming explanation")
    owner_preparation = api.prepare_configuration_improvement_preview(
        owner,
        candidate,
        pypale.ConfigurationImprovementCandidateRequest(
            project_id="demo",
            execution_workspace_id="default",
            machine_id="machine",
            private_inputs=_private_inputs(
                api, root, "/captured/repository-candidate"
            ),
        ),
    )
    try:
        api.commit_configuration_improvement_preview(foreign, owner_preparation)
    except pypale.PaleDiagnosticError as error:
        assert (
            error.diagnostic.code
            == golden["diagnostics"]["preparation_owner_mismatch"]["code"]
        )
    else:
        raise AssertionError("equal coordinates must not replace the private owner token")
    replacement = api.prepare_configuration_improvement_preview(
        owner,
        candidate,
        pypale.ConfigurationImprovementCandidateRequest(
            project_id="demo",
            execution_workspace_id="default",
            machine_id="machine",
            private_inputs=_private_inputs(
                api, root, "/captured/repository-candidate"
            ),
        ),
    )
    api.commit_configuration_improvement_preview(owner, replacement).reject()

    assert (
        _direct_preview(api, root, baseline, profile).reject().disposition
        == pypale.ConfigurationImprovementDisposition.Rejected
    )
    assert (
        _direct_preview(api, root, baseline, profile).cancel().disposition
        == pypale.ConfigurationImprovementDisposition.Cancelled
    )
    assert (
        _direct_preview(api, root, baseline, profile).mark_stale().disposition
        == pypale.ConfigurationImprovementDisposition.Stale
    )

    owner = _explanation(api, baseline, profile)
    preparation = api.prepare_configuration_improvement_preview(
        owner,
        candidate,
        pypale.ConfigurationImprovementCandidateRequest(
            project_id="demo",
            execution_workspace_id="default",
            machine_id="machine",
            private_inputs=_private_inputs(api, root, "/captured/repository-candidate"),
        ),
    )
    start = Barrier(2)

    def commit() -> pypale.ConfigurationImprovementPreview:
        start.wait()
        return api.commit_configuration_improvement_preview(owner, preparation)

    winners: list[pypale.ConfigurationImprovementPreview] = []
    consumed = 0
    with ThreadPoolExecutor(max_workers=2) as workers:
        attempts = [workers.submit(commit) for _ in range(2)]
        for attempt in attempts:
            try:
                winners.append(attempt.result())
            except ValueError:
                consumed += 1
    assert len(winners) == 1 and consumed == 1
    for observation in (lambda: owner.phase, lambda: preparation.summary):
        try:
            observation()
        except ValueError:
            pass
        else:
            raise AssertionError("concurrent commit must consume both original handles")
    winner = winners[0]
    assert json.loads(winner.summary.canonical_bytes) == golden["summary"]
    assert winner.comparison().change_root_count == golden["comparison"]["change_root_count"]
    assert winner.reject().disposition == pypale.ConfigurationImprovementDisposition.Rejected
