from ._native import *
from .builders import WorkspaceModuleBuilder as WorkspaceModuleBuilder
from .builders import build_workspace_module as build_workspace_module
from .models import *

# PyO3 create_exception! has no generated class fragment for the abi3-py39
# boundary. Only this native exception shell is authored here; Diagnostic and
# every native operation remain defined by the generated _native.pyi.
class PaleDiagnosticError(Exception):
    """Admitted diagnostic and compiler authoring evidence, not edit permission."""

    diagnostic: Diagnostic
    diagnostic_bytes: bytes
    authoring_corrections: list[AuthoringCorrection]
    authoring_location: AuthoringSourceLocation | None
