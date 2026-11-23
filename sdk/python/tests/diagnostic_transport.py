"""Reject malformed exception delivery without inventing diagnostic policy."""

from unittest.mock import patch

import pypale


def exercise_diagnostic_transport(api: pypale.Pale) -> None:
    """Host attribute failures escape intact; ordinary diagnostics stay exact."""
    def canonical_failure() -> bytes:
        try:
            api.contract_schema("not-a-selected-schema")
        except pypale.PaleDiagnosticError as error:
            assert error.diagnostic.canonical_bytes == error.diagnostic_bytes
            return error.diagnostic_bytes
        raise AssertionError("unknown schema must fail")

    expected = canonical_failure()
    original = pypale.PaleDiagnosticError.__setattr__
    for denied in ("diagnostic", "diagnostic_bytes", "authoring_corrections", "authoring_location"):
        failure = RuntimeError("host denied exception decoration")

        def set_attribute(self: pypale.PaleDiagnosticError, name: str, value: object) -> None:
            if name == denied:
                raise failure
            original(self, name, value)

        with patch.object(pypale.PaleDiagnosticError, "__setattr__", set_attribute):
            try:
                api.contract_schema("not-a-selected-schema")
            except RuntimeError as error:
                assert error is failure
                assert not isinstance(error, pypale.PaleDiagnosticError)
            else:
                raise AssertionError("the original host exception must escape")
        assert canonical_failure() == expected
