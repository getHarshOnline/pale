"""Exercise read-only control-store inspection through the installed Python ABI."""

import json
from pathlib import Path
from tempfile import TemporaryDirectory

import pypale


def exercise_control_storage(api: pypale.Pale) -> None:
    """Observe both scopes and reject bad coordinates without creating storage."""
    request = pypale.LocalControlStorageInspectionRequest
    store_kind = pypale.LocalControlStoreKind
    state_kind = pypale.LocalControlStorageInspectionStateKind
    with TemporaryDirectory(prefix="pale-python-inspection-") as temporary:
        root = str(Path(temporary).resolve())
        principal = dict(
            runtime_state_root=root,
            principal_id="principal-inspection",
            logical_store_id="store-inspection",
            control_incarnation_id="incarnation-inspection",
        )
        deployment = dict(
            runtime_state_root=root,
            workspace_id="workspace-inspection",
            deployment_binding_ref="deployment-binding-inspection",
            deployment_binding_sha256="0" * 64,
            logical_store_id="store-inspection",
            control_incarnation_id="incarnation-inspection",
        )
        for constructor, fields, kind in (
            (request.principal, principal, store_kind.PrincipalTrust),
            (request.deployment, deployment, store_kind.DeploymentControl),
        ):
            selected = constructor(**fields)
            assert root not in repr(selected)
            observed = api.inspect_local_control_store(selected)
            assert observed.kind == kind
            assert observed.state.kind == state_kind.Missing
            assert observed.state.unresolved is None
            assert observed.state.rejection is None
            assert root not in repr(observed)
            assert list(Path(root).iterdir()) == []

        invalid = [
            request.principal(**{**principal, "runtime_state_root": "relative/root"}),
            request.principal(**{**principal, "principal_id": ""}),
            request.principal(**{**principal, "logical_store_id": ""}),
            request.principal(**{**principal, "control_incarnation_id": ""}),
            request.deployment(**{**deployment, "workspace_id": ""}),
            request.deployment(**{**deployment, "deployment_binding_ref": ""}),
            request.deployment(**{**deployment, "deployment_binding_sha256": "not-a-digest"}),
        ]
        for selected in invalid:
            try:
                api.inspect_local_control_store(selected)
            except pypale.PaleDiagnosticError as error:
                diagnostic = error.diagnostic
                assert diagnostic.code == "local_control_storage_inspection_failed"
                assert diagnostic.stage == pypale.DiagnosticStage.HostObservation
                assert diagnostic.disposition == pypale.DiagnosticDisposition.InvalidInput
                assert diagnostic.retry == pypale.DiagnosticRetry.AfterCorrection
                assert diagnostic.effect == pypale.DiagnosticEffectCertainty.NotApplicable
                assert str(error) == diagnostic.summary
                assert diagnostic.canonical_bytes == error.diagnostic_bytes
                assert api.admit_diagnostic(error.diagnostic_bytes) == error.diagnostic_bytes
                assert root.encode() not in error.diagnostic_bytes
            else:
                raise AssertionError("invalid inspection coordinates must fail closed")
            assert list(Path(root).iterdir()) == []


if __name__ == "__main__":
    exercise_control_storage(pypale.Pale())
    print(json.dumps({"controlStorageInspection": "passed", "mutations": 0}, indent=2))
