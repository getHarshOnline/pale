"""Maturin builds the extension; PyO3's model and renderer own native stubs.

This PEP 517 backend adds the factory-only constructor declarations missing from
the pinned renderer. Wheel owns unpacking, RECORD verification and repacking.
No native library is imported, no generated declaration is patched as text, and
no runtime constructor or SDK operation is changed.
"""

from __future__ import annotations

import ast
import hashlib
import importlib.metadata
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from typing import BinaryIO

import maturin
from wheel.cli.pack import pack
from wheel.cli.unpack import unpack
from wheel.wheelfile import WheelFile


_ROOT = Path(__file__).resolve().parent
_STUB_MEMBER = "pypale/_native.pyi"
_KNOWN_COLLISION_NAME = re.compile(
    r"^(?:[A-Z][A-Za-z\d]*V1[1-9]\d*|[A-Z][A-Za-z\d]*Variant\d+)$",
    flags=re.ASCII,
)
_VERSION_SEGMENT = re.compile(r"V[1-9]\d*", flags=re.ASCII)
_VERSIONED_OWNER = re.compile(r"V[1-9]\d*$", flags=re.ASCII)

# Metadata, requirements, interpreter selection and Maturin flags retain their
# existing owner. These hooks do not claim to have finalized a built package.
get_requires_for_build_wheel = maturin.get_requires_for_build_wheel
get_requires_for_build_editable = maturin.get_requires_for_build_editable
get_requires_for_build_sdist = maturin.get_requires_for_build_sdist
prepare_metadata_for_build_wheel = maturin.prepare_metadata_for_build_wheel
prepare_metadata_for_build_editable = maturin.prepare_metadata_for_build_editable
build_sdist = maturin.build_sdist


def _check_tools() -> None:
    for name, expected in (("maturin", "1.15.0"), ("wheel", "0.45.1")):
        if importlib.metadata.version(name) != expected:
            raise RuntimeError(f"{name} {expected} is required by this build backend")


def _stream_digest(stream: BinaryIO) -> bytes:
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(block)
    return digest.digest()


def _digest(path: Path) -> bytes:
    with path.open("rb") as stream:
        return _stream_digest(stream)


def _native_file(directory: Path) -> Path:
    candidates = [
        path
        for path in directory.iterdir()
        if path.name.startswith("_native.") and path.suffix in (".so", ".pyd")
    ]
    if len(candidates) != 1 or candidates[0].is_symlink() or not candidates[0].is_file():
        raise RuntimeError("expected exactly one regular pypale._native extension")
    return candidates[0]


def _public_declaration_names(tree: ast.Module) -> list[str]:
    names: list[str] = []
    for statement in tree.body:
        if isinstance(statement, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            names.append(statement.name)
        elif isinstance(statement, (ast.Assign, ast.AnnAssign)):
            targets = statement.targets if isinstance(statement, ast.Assign) else [statement.target]
            names.extend(target.id for target in targets if isinstance(target, ast.Name))
    return [name for name in names if name[:1].isupper()]


def _reject_known_collision_names(names: list[str]) -> None:
    known = next((name for name in names if _KNOWN_COLLISION_NAME.fullmatch(name)), None)
    if known is not None:
        raise RuntimeError(
            f"generated native stub retained meaningless collision name {known}"
        )


def _reject_repeated_owner_names(names: list[str]) -> None:
    repeated_version = next(
        (name for name in names if len(_VERSION_SEGMENT.findall(name)) > 1), None
    )
    if repeated_version is not None:
        raise RuntimeError(
            "generated native stub repeats a version segment: " + repeated_version
        )

    versioned_owners = [name for name in names if _VERSIONED_OWNER.search(name)]
    redundant_owner = next(
        (
            name
            for name in names
            for owner in versioned_owners
            if name != owner
            and name.startswith(owner)
            and name[len(owner) : len(owner) + 1].isupper()
        ),
        None,
    )
    if redundant_owner is not None:
        raise RuntimeError(
            "generated native stub repeats its owning declaration: " + redundant_owner
        )


def _numeric_suffix(name: str) -> tuple[str, str] | None:
    split = len(name)
    while split > 0 and "0" <= name[split - 1] <= "9":
        split -= 1
    suffix = name[split:]
    if not suffix or not name[:split] or suffix.startswith("0"):
        return None
    return name[:split], suffix


def _reject_numeric_collision_counters(names: list[str]) -> None:
    declared = set(names)
    suffix_groups: dict[str, set[str]] = {}
    for name in names:
        suffix = _numeric_suffix(name)
        if suffix is not None:
            suffix_groups.setdefault(suffix[0], set()).add(name)
    for base, members in suffix_groups.items():
        if base in declared or len(members) > 1:
            joined = ", ".join(sorted(members))
            raise RuntimeError(
                f"generated native stub uses a numeric collision counter: {joined}"
            )


def _check_stub_declaration_names(path: Path) -> None:
    """Reject generator-shaped public declaration names."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (SyntaxError, UnicodeError) as error:
        raise RuntimeError("generated native stub is not valid UTF-8 Python syntax") from error

    names = _public_declaration_names(tree)
    _reject_known_collision_names(names)
    _reject_repeated_owner_names(names)
    _reject_numeric_collision_counters(names)


def _render_stub(native: Path, output: Path) -> None:
    before = _digest(native)
    # The build helper must run on the host even when Maturin targets another
    # platform. It reads the target binary's metadata without loading its code.
    rustc = subprocess.run(
        ["rustc", "-vV"], check=True, capture_output=True, text=True,
    ).stdout
    hosts = [
        line.removeprefix("host: ")
        for line in rustc.splitlines()
        if line.startswith("host: ")
    ]
    if len(hosts) != 1:
        raise RuntimeError("rustc did not report one host target")
    subprocess.run(
        [
            "cargo", "run", "--locked", "--manifest-path",
            str(_ROOT / "tools/stubgen/Cargo.toml"), "--target", hosts[0],
            "--", str(native), str(output),
        ], check=True,
    )
    if _digest(native) != before:
        raise RuntimeError("native extension changed during stub generation")
    if not output.is_file() or output.is_symlink() or output.stat().st_size == 0:
        raise RuntimeError("stub generator did not produce one regular nonempty stub")
    _check_stub_declaration_names(output)


def _wheel_members(path: Path) -> dict[str, bytes]:
    # WheelFile checks each member against the wheel-owned RECORD as it is read.
    with WheelFile(str(path)) as wheel:
        names = wheel.namelist()
        if len(set(names)) != len(names):
            raise RuntimeError("wheel has duplicate members")
        if sum(name.endswith(".dist-info/RECORD") for name in names) != 1:
            raise RuntimeError("wheel must contain exactly one RECORD member")
        members = {}
        for name in names:
            if not name.endswith("/"):
                with wheel.open(name) as stream:
                    members[name] = _stream_digest(stream)
        return members


def _publish_wheel(source: Path, wheel_directory: str) -> str:
    destination = Path(wheel_directory).resolve()
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".pypale-wheel-", dir=destination) as temporary:
        staged = Path(temporary) / source.name
        shutil.copy2(source, staged)
        os.replace(staged, destination / source.name)
    return source.name


def _maturin_settings(config_settings, output: Path) -> dict:
    """Preserve caller settings while giving this build a private wheel path."""
    arguments = list(maturin.get_maturin_pep517_args(config_settings))
    if any(
        argument in ("-o", "--out") or argument.startswith("--out=")
        for argument in arguments
    ):
        raise RuntimeError("the PyPale build backend owns Maturin's wheel output path")
    settings = dict(config_settings or {})
    settings.pop("build-args", None)
    settings["maturin.build-args"] = [*arguments, "--out", str(output)]
    return settings


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    """Return a wheel only after rendering and verifying its exact native stub."""
    _check_tools()
    with tempfile.TemporaryDirectory(prefix="pypale-wheel-") as temporary:
        staging = Path(temporary)
        maturin_output = staging / "maturin-output"
        maturin_output.mkdir()
        original = staging / maturin.build_wheel(
            str(staging),
            _maturin_settings(config_settings, maturin_output),
            metadata_directory,
        )
        before = _wheel_members(original)
        unpack(str(original), str(staging / "unpacked"))
        roots = list((staging / "unpacked").iterdir())
        if len(roots) != 1:
            raise RuntimeError("wheel unpack did not produce one package root")
        root = roots[0]
        native = _native_file(root / "pypale")
        _render_stub(native, root / _STUB_MEMBER)
        output = staging / "final"
        output.mkdir()
        pack(str(root), str(output), None)
        wheels = list(output.glob("*.whl"))
        if len(wheels) != 1 or wheels[0].name != original.name:
            raise RuntimeError("wheel repack changed the package filename or tags")
        after = _wheel_members(wheels[0])
        record = next(name for name in before if name.endswith(".dist-info/RECORD"))
        allowed = {_STUB_MEMBER, record}
        if {key: value for key, value in before.items() if key not in allowed} != {
            key: value for key, value in after.items() if key not in allowed
        } or _STUB_MEMBER not in after:
            raise RuntimeError(
                "wheel finalization changed content outside its native stub and RECORD"
            )
        return _publish_wheel(wheels[0], wheel_directory)


def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    """Finalize the exact source extension installed by Maturin's editable build.

    Like Maturin itself, this route updates generated source-tree files. It does
    not roll back an older editable extension if a later build step fails. No
    editable wheel is returned on failure, and no old stub is used as fallback.
    """
    _check_tools()
    with tempfile.TemporaryDirectory(prefix="pypale-editable-") as temporary:
        staging = Path(temporary)
        maturin_output = staging / "maturin-output"
        maturin_output.mkdir()
        wheel = staging / maturin.build_editable(
            str(staging),
            _maturin_settings(config_settings, maturin_output),
            metadata_directory,
        )
        _wheel_members(wheel)
        source = _ROOT / "python"
        with WheelFile(str(wheel)) as archive:
            paths = [name for name in archive.namelist() if name.endswith(".pth")]
            if len(paths) != 1 or archive.read(paths[0]).decode().strip() != str(source):
                raise RuntimeError("editable wheel does not point to this SDK's Python source")
        native = _native_file(source / "pypale")
        # Place the temporary output beside its destination for atomic replace.
        with tempfile.TemporaryDirectory(
            prefix=".pypale-stub-", dir=source / "pypale",
        ) as generated:
            stub = Path(generated) / "_native.pyi"
            _render_stub(native, stub)
            os.replace(stub, source / _STUB_MEMBER)
        return _publish_wheel(wheel, wheel_directory)
