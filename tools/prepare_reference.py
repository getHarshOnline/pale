#!/usr/bin/env python3
"""Capture one selected source and rebuild its public reference declarations.

The output is a fresh, regular-file source tree for a documentation generator.
Native SDK checks run in a separate working copy with its own Cargo target.
Source options retain their exact bytes; installed documentation tools remain
the consuming workspace's selection.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import tempfile
from typing import Any


OPTIONS = (
    ("typedoc_options", "typedoc.json", {"plugin", "plugins"}),
    ("typescript_config", "tsconfig.json", set()),
    ("rustdoc_options", "rustdoc.json", {"cargo", "rustc", "rustdoc"}),
)
DECLARATIONS = (
    "sdk/js/index.d.ts",
    "sdk/wasm/index.d.ts",
    "sdk/wasm/dist/pale_wasm.d.ts",
)
GENERATED_SOURCE_FILES = frozenset({
    "sdk/js/index.js",
    "sdk/js/index.d.ts",
})


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def regular_bytes(root: Path, relative: str) -> bytes:
    path = PurePosixPath(relative)
    if path.is_absolute() or not path.parts or ".." in path.parts:
        raise ValueError(f"source member is not confined: {relative}")
    current = root
    for part in path.parts:
        current /= part
        if current.is_symlink():
            raise ValueError(f"source member traverses a symlink: {relative}")
    if not current.is_file():
        raise ValueError(f"source member is not a regular file: {relative}")
    return current.read_bytes()


def selected_files(source: Path) -> list[str]:
    observed_root = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], cwd=source, text=True
    ).strip()
    if Path(observed_root).resolve() != source:
        raise ValueError("--source must name the repository root")
    data = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=source,
    )
    deleted = subprocess.check_output(["git", "ls-files", "-z", "--deleted"], cwd=source)
    absent = {item.decode("utf-8") for item in deleted.split(b"\0") if item}
    return sorted({item.decode("utf-8") for item in data.split(b"\0") if item} - absent)


def inventory(root: Path, files: list[str]) -> dict[str, dict[str, Any]]:
    result = {}
    for relative in files:
        data = regular_bytes(root, relative)
        result[relative] = {
            "sha256": digest(data), "bytes": len(data),
            "mode": root.joinpath(relative).stat().st_mode & 0o777,
        }
    return result


def verify_copied_source(
    root: Path, expected: dict[str, dict[str, Any]],
    generated_outputs: frozenset[str] = frozenset(),
) -> dict[str, dict[str, Any]]:
    """Reject build-copy edits outside the explicitly rebuilt source outputs."""
    observed = inventory(root, sorted(expected))
    changes = {name: value for name, value in observed.items() if value != expected[name]}
    unexpected = changes.keys() - generated_outputs
    if unexpected:
        raise ValueError("copied source changed: " + ", ".join(sorted(unexpected)))
    return changes


def tree_files(root: Path) -> list[str]:
    if root.is_symlink() or not root.is_dir():
        raise ValueError(f"required directory is missing or linked: {root}")
    files = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"prepared tree contains a symlink: {path.relative_to(root)}")
        if path.is_file():
            files.append(path.relative_to(root).as_posix())
        elif not path.is_dir():
            raise ValueError(f"prepared tree contains a special entry: {path.relative_to(root)}")
    return files


def copy_member(source: Path, destination: Path, relative: str) -> None:
    data = regular_bytes(source, relative)
    target = destination / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    target.chmod(source.joinpath(relative).stat().st_mode & 0o777)


def source_options(args: argparse.Namespace) -> dict[str, bytes]:
    options = {}
    for argument, name, forbidden in OPTIONS:
        path = Path(getattr(args, argument)).resolve(strict=True)
        data = path.read_bytes()
        value = json.loads(data)
        if not isinstance(value, dict) or forbidden.intersection(value):
            raise ValueError(f"{name} must be source options without installed tool selections")
        options[name] = data
    return options


def unpack_contracts(archive: Path, destination: Path) -> None:
    """Retain the actual packed Package, rejecting links and escaped members."""
    with tarfile.open(archive, "r:gz") as packed:
        members = packed.getmembers()
        seen = set()
        for member in members:
            path = PurePosixPath(member.name)
            if (
                path.is_absolute()
                or ".." in path.parts
                or not path.parts
                or path.parts[0] != "package"
                or not (member.isfile() or member.isdir())
                or path in seen
            ):
                raise ValueError(f"invalid contracts archive member: {member.name}")
            seen.add(path)
        for member in members:
            relative = PurePosixPath(member.name).relative_to("package")
            target = destination.joinpath(*relative.parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                stream = packed.extractfile(member)
                if stream is None:
                    raise ValueError(f"archive member has no file bytes: {member.name}")
                with stream:
                    target.write_bytes(stream.read())


class Commands:
    """Keep the real command result and its log in the preparation report."""

    def __init__(self, root: Path, evidence: Path) -> None:
        self.root = root
        self.evidence = evidence
        self.results: list[dict[str, Any]] = []
        self.environment = os.environ.copy()
        self.environment["CARGO_TARGET_DIR"] = str(root / "target")
        self.environment["CARGO_INCREMENTAL"] = "0"
        self.environment["CARGO_NET_OFFLINE"] = "true"
        self.environment.pop("RUSTC_WRAPPER", None)
        self.environment.pop("RUSTC_WORKSPACE_WRAPPER", None)

    def run(self, label: str, directory: str, arguments: list[str]) -> str:
        log = self.evidence / f"{len(self.results):02d}-{label}.log"
        attempt = {
            "label": label, "cwd": directory, "arguments": list(arguments),
            "exit_code": None, "error": None, "log": log.name,
            "log_sha256": None,
        }
        self.results.append(attempt)
        print(f"Running {label}", flush=True)
        try:
            with log.open("wb") as output:
                result = subprocess.run(
                    arguments, cwd=self.root / directory, env=self.environment,
                    stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                    check=False,
                )
            attempt["exit_code"] = result.returncode
        except BaseException as error:
            attempt["error"] = {"type": type(error).__name__, "message": str(error)}
            raise
        finally:
            if log.is_file():
                attempt["log_sha256"] = digest(log.read_bytes())
        data = log.read_bytes()
        if result.returncode:
            raise RuntimeError(f"{label} failed ({result.returncode}); read {log}")
        return data.decode("utf-8")


def prepare(args: argparse.Namespace) -> None:
    source = Path(args.source).resolve(strict=True)
    output = Path(args.output).absolute()
    if output.exists() or output.is_symlink():
        raise ValueError("--output must be a fresh directory")
    if not output.parent.is_dir():
        raise ValueError("the output parent must already exist")
    options = source_options(args)
    files = selected_files(source)
    before = inventory(source, files)
    if any(name in before for name in (*options, "preparation.json")) or any(
        name == ".reference-preparation" or name.startswith(".reference-preparation/")
        for name in before
    ):
        raise ValueError("source already contains a reserved preparation output")
    output.mkdir()
    evidence = output / ".reference-preparation"
    evidence.mkdir()
    report: dict[str, Any] = {
        "status": "incomplete", "selected_source": str(source),
        "selected_files": before,
        "source_options": {name: {"sha256": digest(data), "bytes": len(data)} for name, data in options.items()},
        "compiler_cache": "disabled; separate working-copy target; incremental off",
    }
    report_path = output / "preparation.json"
    try:
        with tempfile.TemporaryDirectory(prefix="pale-reference-build-", dir=output.parent) as temporary:
            build = Path(temporary)
            for relative in files:
                copy_member(source, build, relative)
            verify_copied_source(build, before)
            commands = Commands(build, evidence)
            report["commands"] = commands.results
            report["tools"] = {}
            for program in ("node", "npm", "cargo", "rustc", "wasm-pack"):
                executable = shutil.which(program)
                if executable is None:
                    raise ValueError(f"required build tool is unavailable: {program}")
                executable_path = Path(executable).resolve(strict=True)
                version = commands.run(f"{program}-version", ".", [executable, "--version"])
                report["tools"][program] = {
                    "path": str(executable_path), "sha256": digest(executable_path.read_bytes()),
                    "version": version.strip(),
                }
            if int(report["tools"]["node"]["version"].lstrip("v").split(".")[0]) < 22:
                raise ValueError("native SDK preparation requires Node.js 22 or later")
            for directory, label in (("sdk/js", "node-sdk"), ("sdk/js/contracts", "contracts")):
                commands.run(f"{label}-dependencies", directory, ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"])
                commands.run(f"{label}-tests", directory, ["npm", "test"])
            commands.run("contracts-freshness", "sdk/js/contracts", ["npm", "run", "check:fresh"])
            artifacts = build / "reference-artifacts"
            artifacts.mkdir()
            pack_output = commands.run("contracts-pack", "sdk/js/contracts", [
                "npm", "pack", "--json", "--ignore-scripts", "--pack-destination", str(artifacts),
            ])
            packed = json.loads(pack_output)
            if len(packed) != 1 or Path(packed[0]["filename"]).name != packed[0]["filename"]:
                raise ValueError("contracts packing did not return one confined archive")
            archive = artifacts / packed[0]["filename"]
            for name in ("build", "test:wasm", "test:abi"):
                commands.run(f"wasm-{name.replace(':', '-')}", "sdk/wasm", ["npm", "run", name])
            commands.run("wasm-packed-consumer", "sdk/wasm", [
                "node", "tests/package_consumer.mjs", "--contracts", str(archive),
                "--typescript", str(build / "sdk/js/node_modules/typescript/bin/tsc"),
            ])
            native_artifacts = []
            for directory, pattern in (("sdk/js", "*.node"), ("sdk/wasm/dist", "*.wasm")):
                produced = [path.relative_to(build).as_posix() for path in (build / directory).glob(pattern)]
                if not produced:
                    raise ValueError(f"native checks produced no SDK artifacts in {directory}")
                native_artifacts.extend(produced)
            report["native_artifacts"] = inventory(build, sorted(native_artifacts))
            if selected_files(source) != files or inventory(source, files) != before:
                raise ValueError("selected source changed during preparation; choose a stable source and rerun")
            report["generated_source_changes"] = verify_copied_source(
                build, before, GENERATED_SOURCE_FILES | frozenset(native_artifacts),
            )
            for relative in files:
                copy_member(build, output, relative)
            for relative in DECLARATIONS:
                copy_member(build, output, relative)
            for relative in tree_files(build / "sdk/wasm/dist"):
                copy_member(build / "sdk/wasm/dist", output / "sdk/wasm/dist", relative)
            for dependency in ("@types/node", "undici-types"):
                origin = build / "sdk/js/node_modules" / dependency
                for relative in tree_files(origin):
                    copy_member(origin, output / "node_modules" / dependency, relative)
            unpack_contracts(archive, output / "sdk/wasm/node_modules/@rtepl/pale-contracts")
            regular_bytes(output, "sdk/wasm/node_modules/@rtepl/pale-contracts/package.json")
            regular_bytes(output, "sdk/wasm/node_modules/@rtepl/pale-contracts/dist/index.d.ts")
            for name, data in options.items():
                (output / name).write_bytes(data)
            report["declarations"] = inventory(output, sorted(set(DECLARATIONS) | {
                "sdk/wasm/dist/" + relative for relative in tree_files(output / "sdk/wasm/dist")
                if relative.endswith(".d.ts")
            }))
            report["contracts_archive"] = {"name": archive.name, "sha256": digest(archive.read_bytes()), "bytes": archive.stat().st_size}
            shutil.copyfile(archive, evidence / archive.name)
            prepared_files = [name for name in tree_files(output) if not name.startswith(".reference-preparation/")]
            report["prepared_files"] = inventory(output, prepared_files)
            encoded = json.dumps(report["prepared_files"], sort_keys=True, separators=(",", ":")).encode()
            report["prepared_tree_sha256"] = digest(encoded)
            report["status"] = "complete"
    except BaseException as error:
        report["status"] = "failed"
        report["failure"] = str(error)
        raise
    finally:
        report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(f"Prepared {output}; identity {report['prepared_tree_sha256']}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, help="selected Pale Git working-copy root")
    parser.add_argument("--output", required=True, help="fresh prepared-source directory")
    for argument, _, _ in OPTIONS:
        parser.add_argument("--" + argument.replace("_", "-"), required=True)
    prepare(parser.parse_args())


if __name__ == "__main__":
    main()
