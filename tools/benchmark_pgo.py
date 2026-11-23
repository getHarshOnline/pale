#!/usr/bin/env python3
"""Generate a checksum-bound ordinary-versus-PGO SQLite benchmark comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


BENCH_PACKAGE = "pale-adapter-control-storage"
BENCH_TARGET = "control_sqlite"
COMPLETE_MARKER = "#pale_control_sqlite_report_complete_v1"


def output(command: list[str], *, cwd: Path) -> str:
    """Run one metadata command and return normalized UTF-8 stdout."""
    completed = subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)
    return completed.stdout.strip()


def llvm_major(text: str) -> int:
    """Extract the leading LLVM major without accepting an ambiguous version."""
    match = re.search(r"(?:LLVM version:|LLVM version|version)\s+(\d+)(?:\.\d+)+", text)
    if match is None:
        raise ValueError(f"cannot determine LLVM version from: {text!r}")
    return int(match.group(1))


def require_matching_profdata(source_root: Path, program: str) -> tuple[str, str]:
    """Reject profile tools whose LLVM major differs from the active Rust compiler."""
    rustc = output(["rustc", "-vV"], cwd=source_root)
    profdata = output([program, "--version"], cwd=source_root)
    if llvm_major(rustc) != llvm_major(profdata):
        raise SystemExit(
            "llvm-profdata major does not match rustc LLVM: "
            f"rustc={llvm_major(rustc)} llvm-profdata={llvm_major(profdata)}"
        )
    return rustc, profdata


def sha256(path: Path) -> str:
    """Hash one regular evidence artifact."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_benchmark(source_root: Path, target_dir: Path, rustflags: list[str]) -> Path:
    """Build the exact benchmark target and return Cargo's executable artifact."""
    environment = os.environ.copy()
    environment["RUSTFLAGS"] = " ".join(rustflags)
    command = [
        "cargo",
        "build",
        "--locked",
        "--offline",
        "--profile",
        "bench",
        "--package",
        BENCH_PACKAGE,
        "--bench",
        BENCH_TARGET,
        "--features",
        "bench-memory",
        "--target-dir",
        str(target_dir),
        "--message-format",
        "json-render-diagnostics",
    ]
    completed = subprocess.run(
        command,
        cwd=source_root,
        env=environment,
        check=True,
        capture_output=True,
        text=True,
    )
    executables = []
    for line in completed.stdout.splitlines():
        message = json.loads(line)
        executable = message.get("executable")
        target = message.get("target", {})
        if executable and target.get("name") == BENCH_TARGET:
            executables.append(Path(executable))
    if len(executables) != 1:
        raise SystemExit(f"expected one benchmark executable, found {executables}")
    return executables[0]


def run_benchmark(binary: Path, report: Path, extra_environment: dict[str, str]) -> None:
    """Run one complete retained workload with an exact executable coordinate."""
    environment = os.environ.copy()
    environment.pop("PALE_SQLITE_BENCH_SMOKE", None)
    environment.pop("PALE_BENCH_PGO", None)
    environment.update(extra_environment)
    environment["PALE_SQLITE_BENCH_BINARY_SHA256"] = sha256(binary)
    with report.open("wb") as destination:
        subprocess.run([str(binary)], env=environment, check=True, stdout=destination)
    if not report.read_text(encoding="utf-8").splitlines()[-1].startswith(COMPLETE_MARKER):
        raise SystemExit(f"benchmark report is not complete: {report}")


def receipt_member(path: Path) -> dict[str, object]:
    """Describe one regular comparison artifact without interpreting its timings."""
    return {"path": path.name, "sha256": sha256(path), "bytes": path.stat().st_size}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=Path.cwd())
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--llvm-profdata", default="llvm-profdata")
    parser.add_argument("--check-toolchain", action="store_true")
    args = parser.parse_args()

    source_root = args.source_root.resolve(strict=True)
    rustc, profdata = require_matching_profdata(source_root, args.llvm_profdata)
    if args.check_toolchain:
        print(json.dumps({"rustc": rustc.splitlines()[0], "llvm_profdata": profdata}))
        return

    output_root = args.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=False)
    build_root = output_root / "build"
    profiles = output_root / "profiles"
    profiles.mkdir()

    baseline = build_benchmark(source_root, build_root / "baseline", [])
    run_benchmark(baseline, output_root / "baseline.tsv", {})

    instrumented = build_benchmark(
        source_root,
        build_root / "instrumented",
        [f"-Cprofile-generate={profiles}"],
    )
    run_benchmark(
        instrumented,
        output_root / "instrumented.tsv",
        {"LLVM_PROFILE_FILE": str(profiles / "%m_%p.profraw")},
    )
    raw_profiles = sorted(profiles.glob("*.profraw"))
    if not raw_profiles:
        raise SystemExit("instrumented workload emitted no raw profiles")
    merged = output_root / "merged.profdata"
    subprocess.run(
        [args.llvm_profdata, "merge", "-o", str(merged), *map(str, raw_profiles)],
        cwd=source_root,
        check=True,
    )

    optimized = build_benchmark(
        source_root,
        build_root / "profile-use",
        [f"-Cprofile-use={merged}", "-Cllvm-args=-pgo-warn-missing-function"],
    )
    run_benchmark(optimized, output_root / "profile-use.tsv", {})

    baseline_copy = output_root / "baseline-binary"
    optimized_copy = output_root / "profile-use-binary"
    shutil.copyfile(baseline, baseline_copy)
    shutil.copyfile(optimized, optimized_copy)
    members = [
        receipt_member(path)
        for path in [
            baseline_copy,
            optimized_copy,
            output_root / "baseline.tsv",
            output_root / "instrumented.tsv",
            output_root / "profile-use.tsv",
            merged,
        ]
    ]
    receipt = {
        "evidence_class": "local_unselected_pgo_comparison",
        "rustc": rustc,
        "llvm_profdata": profdata,
        "threshold_selected": False,
        "release_evidence": False,
        "members": members,
    }
    (output_root / "receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
