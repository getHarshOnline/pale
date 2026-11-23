"""Check the installed wheel's public types with positive and negative consumers."""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

import pypale

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--pyright", default="pyright", help="installed Pyright 1.1.408 executable")
arguments = parser.parse_args()
checker = shutil.which(arguments.pyright)
assert checker is not None, "Pyright 1.1.408 must be installed in the test-tool environment"
assert subprocess.check_output([checker, "--version"], text=True).splitlines()[0] == "pyright 1.1.408"
tests = Path(__file__).resolve().parent
installed = Path(pypale.__file__).resolve()
assert not installed.is_relative_to(tests.parent), "check an installed wheel, not source imports"

with TemporaryDirectory(prefix="pale-python-types-") as temporary:
    root = Path(temporary)
    shutil.copyfile(tests / "typing_consumer.py", root / "consumer.py")
    configuration = root / "pyrightconfig.json"
    configuration.write_text(json.dumps({
        "include": ["consumer.py"], "typeCheckingMode": "strict",
        "extraPaths": [], "pythonVersion": "3.9",
    }, indent=2) + "\n", encoding="utf-8")

    def check() -> dict:
        """Ask the selected checker about the exact installed interpreter."""
        result = subprocess.run(
            [checker, "--pythonpath", sys.executable, "--project", str(configuration), "--outputjson"],
            cwd=root, text=True, capture_output=True, check=False,
        )
        assert result.returncode in (0, 1), result.stderr
        return json.loads(result.stdout)

    accepted = check()
    assert accepted["summary"]["errorCount"] == 0, accepted["generalDiagnostics"]
    with (root / "consumer.py").open("a", encoding="utf-8") as target:
        target.write(
            "\ndef wrong_observation_cursor(snapshot: ObservationSnapshot, profile: EffectiveProfileCompilation, workspace: PortableWorkspaceCompilation, subject: ObservationSubject, cursor: EffectiveAuthoringContinuation) -> None:\n"
            "    snapshot.inspect_profile(profile, workspace, subject, 1, 1, cursor)\n"
            "\ndef invalid(error: PaleDiagnosticError) -> str:\n"
            "    return error.diagnostic_bytes\n"
            "\ndef missing(error: PaleDiagnosticError) -> bytes:\n"
            "    return error.missing_diagnostic_bytes\n"
            "\ndef wrong_correction(error: PaleDiagnosticError) -> bytes:\n"
            "    return error.authoring_corrections[0].expected_source\n"
            "\ndef missing_correction(error: PaleDiagnosticError) -> str:\n"
            "    return error.authoring_corrections[0].write_authority\n"
            "\ndef nullable_location(error: PaleDiagnosticError) -> str:\n"
            "    return error.authoring_location.source_label\n"
            "\ndef wrong_phase(client: Pale, explanation: ConfigurationImprovementExplanation) -> ConfigurationImprovementEvaluation:\n"
            "    return client.evaluate_configuration_improvement(explanation)\n"
            "\ndef wrong_row(row: EffectiveProfileEntry) -> str:\n"
            "    detail = row.detail\n"
            "    if isinstance(detail, EffectiveProfileRepositoryDetail):\n"
            "        return detail.selection_id\n"
            "    return ''\n"
            "\ndef wrong_evidence(value: EffectiveProfileExplanation) -> str:\n"
            "    evidence = value.evidence\n"
            "    if isinstance(evidence, EffectiveProfileConfigurationEvidence):\n"
            "        return evidence.selection\n"
            "    return ''\n"
            "\ndef wrong_requirement(value: RequirementValue) -> str:\n"
            "    if isinstance(value, SatisfiedRequirementDisposition):\n"
            "        return value.reason\n"
            "    return ''\n"
            "\ndef wrong_provider(value: ProviderValue) -> str:\n"
            "    if isinstance(value, EffectiveProfileMissingBindingProviderDisposition):\n"
            "        return value.provider\n"
            "    return ''\n"
            "\ndef wrong_finding(value: FindingValue) -> str:\n"
            "    if isinstance(value, EffectiveProfileNativeRealizationUnknownFinding):\n"
            "        return value.operation_id\n"
            "    return ''\n"
            "\ndef wrong_reason(value: PathReasonValue) -> str:\n"
            "    if isinstance(value, EffectiveProfileDelegationEdgeReason):\n"
            "        return value.contract_id\n"
            "    return ''\n"
            "\ndef wrong_declared_selector(value: EffectiveProfileDeclaredExactSelector) -> str:\n"
            "    return value.package_id\n"
            "\ndef wrong_declared_selection(value: EffectiveProfileDeclaredModeDefaultSelection) -> str:\n"
            "    return value.selector\n"
            "\ndef wrong_selection(value: SelectionValue) -> str:\n"
            "    if isinstance(value, EffectiveProfileRunPolicySelection):\n"
            "        return value.package_id\n"
            "    return ''\n"
            "\ndef wrong_participant(row: EffectiveProfileEntry) -> str:\n"
            "    detail = row.detail\n"
            "    if isinstance(detail, EffectiveProfileResponsibilityDetail):\n"
            "        participant = detail.participant\n"
            "        if isinstance(participant, EffectiveProfileApplicationParticipant):\n"
            "            return participant.engine_id\n"
            "    return ''\n"
            "\ndef wrong_subject(row: EffectiveProfileEntry) -> str:\n"
            "    detail = row.detail\n"
            "    if isinstance(detail, EffectiveProfileResponsibilityDetail):\n"
            "        subject = detail.subject\n"
            "        if isinstance(subject, WorkspaceImportSubjectValue):\n"
            "            return subject.module_id\n"
            "    return ''\n"
            "\ndef missing_terminal(row: EffectiveProfileEntry) -> str:\n"
            "    detail = row.detail\n"
            "    if isinstance(detail, EffectiveProfileExecutionPathDetail):\n"
            "        return detail.terminal_provider_binding.provider_binding_id\n"
            "    return ''\n"
        )
    rejected = check()
    consumer_lines = (root / "consumer.py").read_text(encoding="utf-8").splitlines()
    for function_name, expected_rule in (
        ("wrong_observation_cursor", "reportArgumentType"),
        ("wrong_correction", "reportReturnType"),
        ("missing_correction", "reportAttributeAccessIssue"),
        ("nullable_location", "reportOptionalMemberAccess"),
    ):
        declaration = next(index for index, line in enumerate(consumer_lines) if line.startswith(f"def {function_name}("))
        assert any(
            entry.get("rule") == expected_rule and entry["range"]["start"]["line"] == declaration + 1
            for entry in rejected["generalDiagnostics"]
        ), rejected["generalDiagnostics"]
    rules = {entry.get("rule") for entry in rejected["generalDiagnostics"]}
    assert "reportReturnType" in rules, rejected["generalDiagnostics"]
    assert "reportAttributeAccessIssue" in rules, rejected["generalDiagnostics"]
    assert "reportArgumentType" in rules, rejected["generalDiagnostics"]
    assert "reportOptionalMemberAccess" in rules, rejected["generalDiagnostics"]
    attribute_errors = [entry["message"] for entry in rejected["generalDiagnostics"] if entry.get("rule") == "reportAttributeAccessIssue"]
    for field, owner in (
        ("selection_id", "EffectiveProfileRepositoryDetail"),
        ("engine_id", "EffectiveProfileApplicationParticipant"),
        ("module_id", "WorkspaceImportSubjectValue"),
        ("package_id", "EffectiveProfileRunPolicySelection"),
        ("package_id", "EffectiveProfileDeclaredExactSelector"),
        ("selector", "EffectiveProfileDeclaredModeDefaultSelection"),
        ("selection", "EffectiveProfileConfigurationEvidence"),
        ("reason", "SatisfiedRequirementDisposition"),
        ("provider", "EffectiveProfileMissingBindingProviderDisposition"),
        ("operation_id", "EffectiveProfileNativeRealizationUnknownFinding"),
        ("contract_id", "EffectiveProfileDelegationEdgeReason"),
    ):
        assert any(field in message and owner in message for message in attribute_errors), rejected["generalDiagnostics"]
    print(json.dumps({
        "installedPackage": str(installed), "pyright": "1.1.408",
        "positiveErrors": accepted["summary"]["errorCount"],
        "wrongTypeRejected": True, "missingAttributeRejected": True,
        "wrongPhaseRejected": True,
        "wrongEntryVariantRejected": True, "wrongParticipantVariantRejected": True,
        "wrongSubjectVariantRejected": True, "intrinsicNullabilityChecked": True,
        "wrongSelectionVariantRejected": True,
        "wrongEvidenceVariantRejected": True, "wrongRequirementVariantRejected": True,
        "wrongProviderVariantRejected": True, "wrongFindingVariantRejected": True,
        "wrongPathReasonVariantRejected": True,
    }, indent=2))
