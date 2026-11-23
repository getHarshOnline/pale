"""Verify every generated factory restriction against the installed wheel.

Runtime rejection and strict static rejection are separate checks. Any, casts,
ignored diagnostics and bottom-valued expressions are not security boundaries.
"""

from __future__ import annotations

import argparse
import ast
import json
from pathlib import Path
import subprocess
import sys
import tempfile

import pypale
from pypale import _native


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pyright", required=True)
    args = parser.parse_args()
    stub = Path(_native.__file__).with_name("_native.pyi")
    classes = [node for node in ast.parse(stub.read_text()).body if isinstance(node, ast.ClassDef)]
    factory_names = []
    for cls in classes:
        constructors = [node for node in cls.body if isinstance(node, ast.FunctionDef) and node.name in ("__new__", "__init__")]
        assert constructors, f"implicit constructor remains for {cls.name}"
        if any(
            len(method.args.posonlyargs) == 2
            and method.args.posonlyargs[1].arg == "_factory_only"
            and ast.unparse(method.args.posonlyargs[1].annotation) == "NoReturn"
            and ast.unparse(method.returns) == "NoReturn"
            for method in constructors
        ):
            factory_names.append(cls.name)
    assert factory_names, "the installed stub has no factory-only restrictions"
    for name in factory_names:
        cls = getattr(pypale, name)
        assert cls is getattr(_native, name)
        for construct in (lambda: cls(), lambda: cls(None), lambda: cls.__new__(cls), lambda: cls.__new__(cls, None)):
            try:
                construct()
            except TypeError:
                pass
            else:
                raise AssertionError(f"{name} accepted direct runtime construction")
    # Real constructor, enum members and pure named factory retain their behavior.
    pypale.CompilationDocument("fixture", b"value")
    assert pypale.OperationExposure.SemanticOnly is not None
    assert pypale.LocalControlStorageInspectionRequest.principal(
        runtime_state_root="state", principal_id="principal",
        logical_store_id="store", control_incarnation_id="incarnation",
    ) is not None

    with tempfile.TemporaryDirectory(prefix="pypale-constructor-types-") as temporary:
        root = Path(temporary)
        positive = root / "positive.py"
        positive.write_text(
            "import pypale as api\n"
            "document = api.CompilationDocument('fixture', b'value')\n"
            "exposure = api.OperationExposure.SemanticOnly\n"
            "request = api.LocalControlStorageInspectionRequest.principal(runtime_state_root='state', principal_id='principal', logical_store_id='store', control_incarnation_id='incarnation')\n"
        )
        negative = root / "negative.py"
        lines = ["import pypale as api"]
        expected_lines = set()
        for index, name in enumerate(factory_names):
            for form, expression in enumerate((f"api.{name}()", f"api.{name}(None)", f"api.{name}.__new__(api.{name})", f"api.{name}.__new__(api.{name}, None)")):
                # One call per function prevents NoReturn from hiding later cases.
                lines.extend((f"def rejected_{index}_{form}() -> None:", f"    {expression}"))
                expected_lines.add(len(lines) - 1)
        negative.write_text("\n".join(lines) + "\n")
        config = root / "pyrightconfig.json"
        config.write_text(json.dumps({
            "include": [str(positive), str(negative)], "typeCheckingMode": "strict",
            "pythonVersion": "3.9", "pythonPlatform": "All",
        }))
        run = subprocess.run(
            [args.pyright, "--project", str(config), "--pythonpath", sys.executable, "--outputjson"],
            capture_output=True, text=True, timeout=180, check=False,
        )
        report = json.loads(run.stdout)
        assert report["version"] == "1.1.408", report["version"]
        diagnostics = report["generalDiagnostics"]
        assert run.returncode == 1, run.stdout + run.stderr
        assert len(diagnostics) == len(expected_lines), run.stdout
        assert {item["range"]["start"]["line"] for item in diagnostics} == expected_lines, run.stdout
        assert all(Path(item["file"]) == negative and item["severity"] == "error" for item in diagnostics), run.stdout
    print(f"factory classes={len(factory_names)}; explicit constructors={len(classes) - len(factory_names)}; runtime TypeErrors={4 * len(factory_names)}; strict expected errors={4 * len(factory_names)}; positive controls passed")


if __name__ == "__main__":
    main()
