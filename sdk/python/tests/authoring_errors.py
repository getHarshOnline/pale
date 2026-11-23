"""Project the existing compiler/LSP rejected-input fixture through real Python errors."""

import gc
from pathlib import Path

import pypale


def exercise_authoring_errors(api: pypale.Pale) -> None:
    """Retain compiler evidence without applying its suggested edit."""
    fixture = Path(__file__).resolve().parents[3] / "interfaces/lsp/tests/fixtures/authoring/visibility-widening"
    documents = {source.name: source.read_bytes() for source in sorted(fixture.iterdir())}
    try:
        api.compile_portable_workspace("pale-workspace.toml", list(documents.items()))
    except pypale.PaleDiagnosticError as error:
        assert error.diagnostic.code == "workspace_semantics_rejected"
        assert hasattr(error, "authoring_corrections"), "API correction sidecar must survive the error boundary"
        assert len(error.authoring_corrections) == 1
        correction = error.authoring_corrections[0]
        assert isinstance(correction, pypale.AuthoringCorrection)
        assert correction.kind == pypale.AuthoringCorrectionKind.NarrowImportVisibility
        assert correction.source_label == "consumer-module.toml"
        expected_path = [("field", "module", None), ("field", "imports", None), ("index", None, 0), ("field", "visibility_ceiling", None)]
        assert [(step.kind, step.field_value, step.index_value) for step in correction.path] == expected_path
        assert correction.expected_source == '"public"'
        assert correction.replacement_source == '"protected"'
        assert correction.proof_subject.kind.name == "export"
        assert correction.proof_subject.value == "candidate::public-api"
        assert correction.proof_subject.authored_value == "public-api"
        assert correction.diagnostic_identity == error.diagnostic.identity
        assert error.authoring_location is not None
        assert error.authoring_location.source_label == correction.source_label
        assert [(step.kind, step.field_value, step.index_value) for step in error.authoring_location.path] == expected_path
        assert error.diagnostic_bytes == error.diagnostic.canonical_bytes
        assert str(fixture).encode() not in error.diagnostic_bytes
        assert b'"authoring_corrections"' not in error.diagnostic_bytes
    else:
        raise AssertionError("canonical visibility widening must be rejected")
    # Python clears the caught exception; copied evidence remains useful, not authoritative.
    gc.collect()
    path_copy = correction.path
    path_copy.clear()
    assert [(step.kind, step.field_value, step.index_value) for step in correction.path] == expected_path
    assert correction.expected_source == '"public"'
    try:
        correction.expected_source = '"changed"'
    except AttributeError:
        pass
    else:
        raise AssertionError("copied compiler correction must be frozen")
    try:
        pypale.AuthoringCorrection()
    except TypeError:
        pass
    else:
        raise AssertionError("compiler corrections are factory-only values")

    invalid = dict(documents)
    invalid["pale-workspace.toml"] = invalid["pale-workspace.toml"].replace(b"schema_version = 1", b"schema_version = 2", 1)
    try:
        api.compile_portable_workspace("pale-workspace.toml", list(invalid.items()))
    except pypale.PaleDiagnosticError as error:
        assert error.diagnostic.code == "workspace_contract_rejected"
        assert error.authoring_corrections == []
        assert error.authoring_location is not None
        assert error.authoring_location.source_label == "pale-workspace.toml"
        assert [step.field_value for step in error.authoring_location.path] == ["workspace", "schema_version"]
    else:
        raise AssertionError("invalid schema major must be rejected")
    try:
        api.contract_schema("not-a-selected-schema")
    except pypale.PaleDiagnosticError as error:
        assert error.authoring_corrections == []
        assert error.authoring_location is None
    else:
        raise AssertionError("unknown contract must be rejected")
    assert {source.name: source.read_bytes() for source in sorted(fixture.iterdir())} == documents


if __name__ == "__main__":
    exercise_authoring_errors(pypale.Pale())
    print("authoring error sidecars: correction, location-only, absent, retained copy PASS")
