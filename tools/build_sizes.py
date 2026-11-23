"""Adapt native Pale build measurements for the vault tool's existing reports.

The filesystem policy lives in Pale's confined native adapter. This transitional
consumer requires an explicitly selected executable; it never scans a tree or
builds/installs Pale itself. Unavailable native inspection is an incomplete
result, never an empty tree. Doctor's other unavailable checks do not invalidate
an independently completed build measurement.
"""

from dataclasses import dataclass
import json
import os
from pathlib import Path
import subprocess


@dataclass(frozen=True)
class BuildSize:
    """Logical file lengths, not allocated or reclaimable disk space.

    Hard links count per entry; symlinks are excluded. A partial count is not a
    whole-tree total. Nested or overlapping roots must not be added together.
    """

    root: str
    logical_bytes: int | None
    file_count: int
    skipped_symlinks: int
    complete: bool
    errors: tuple[str, ...]


def measure(root: Path, executable: Path | None = None) -> BuildSize:
    """Consume the exact native doctor result for one caller-selected tree."""
    root = Path(os.path.abspath(os.path.expanduser(str(root))))
    try:
        if executable is None or not Path(executable).is_absolute():
            raise ValueError("Supply --pale-executable with the absolute path of a native Pale executable supporting doctor.")
        process = subprocess.run(
            [str(executable), "--result-format", "json", "doctor", "--build-root", str(root)],
            capture_output=True, text=True, timeout=120, check=False,
        )
        if process.returncode not in (0, 1):
            raise ValueError(f"Native doctor exited {process.returncode}; inspect this executable with doctor before retrying.")
        envelope = json.loads(process.stdout)
        if not isinstance(envelope, dict) or envelope.get("completion") != "complete":
            raise ValueError("Native doctor did not return a complete diagnostic report.")
        reports = envelope["result"]["builds"]
        if len(reports) != 1 or reports[0]["root"] != str(root):
            raise ValueError("Native doctor returned a different build selection.")
        report = reports[0]
        total = report["logical_bytes"]
        counts = (report["file_count"], report["skipped_symlinks"])
        errors = report["errors"]
        if (total is not None and (type(total) is not int or total < 0)
                or any(type(count) is not int or count < 0 for count in counts)
                or type(report["complete"]) is not bool
                or not isinstance(errors, list) or any(not isinstance(error, str) for error in errors)
                or report["complete"] and (total is None or errors)
                or not report["complete"] and not errors):
            raise ValueError("Native doctor returned an inconsistent build observation.")
        return BuildSize(str(root), total, *counts, report["complete"], tuple(errors))
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        return BuildSize(str(root), None, 0, 0, False, (str(exc),))
