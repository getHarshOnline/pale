"""Read Package authoring data through Pale's compiler-owned reader."""

import json
from pathlib import Path
import subprocess


def package_command(root, *arguments):
    source = Path(__file__).resolve().parents[1]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "--offline", "--manifest-path",
         str(source / "interfaces/cli/Cargo.toml"), "--", "--result-format", "json",
         "packages", *arguments, "--root", str(Path(root).resolve())],
        cwd=source, capture_output=True, text=True, check=False,
    )
    if result.returncode:
        raise ValueError(result.stderr.strip() or "Pale rejected the Package sources")
    return json.loads(result.stdout)["result"]


def read_packages(root):
    return package_command(root, "show")
