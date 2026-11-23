#!/usr/bin/env python3
"""Verify root-owned cross-component composition evidence.

This build-time tool compares bytes and repository topology only. It never
reimplements API eligibility, contract admission, diagnostics, or runtime
authority.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path


MANIFEST_NAME = "manifest.json"
ONE_SHOT_MEMBERS = (
    "one-shot/packages/engine/package.toml",
    "one-shot/packages/engine/role.md",
    "one-shot/packages/engine/roles.toml",
    "one-shot/private/capabilities.toml",
    "one-shot/private/distribution-lock.toml",
    "one-shot/private/machine-binding.toml",
    "one-shot/workspace/composition.toml",
    "one-shot/workspace/pale-workspace.toml",
    "one-shot/workspace/workspace-resolution.lock.toml",
    "one-shot/workspace/workspace-universe.lock.toml",
)
CROSS_LEAF_PROJECTIONS = {
    Path("crates/pale-application/tests/fixtures/cross-leaf.v1"): ONE_SHOT_MEMBERS,
    Path("sdk/rust/tests/fixtures/cross-leaf.v1"): None,
    Path("interfaces/cli/tests/fixtures/cross-leaf.v1"): None,
    Path("interfaces/mcp/tests/fixtures/cross-leaf.v1"): (
        "improvement-journey.json",
        *ONE_SHOT_MEMBERS,
    ),
    Path("sdk/js/tests/fixtures/cross-leaf.v1"): None,
    Path("sdk/go/tests/fixtures/cross-leaf.v1"): None,
    Path("sdk/python/tests/fixtures/cross-leaf.v1"): None,
    Path("sdk/wasm/tests/fixtures/cross-leaf.v1"): (
        "bundle-identity.json",
        "contract-diagnostic.json",
        "native-action-journey.json",
        "portable-journey.json",
        "portable/pale-workspace.toml",
        "portable/resolution.lock.toml",
        "portable/universe.lock.toml",
    ),
    Path("interfaces/acp/tests/fixtures/cross-leaf.v1"): (
        "improvement-journey.json",
        *ONE_SHOT_MEMBERS,
    ),
    Path("interfaces/lsp/tests/fixtures/cross-leaf.v1"): (
        "effective-profile-wire.json",
        *ONE_SHOT_MEMBERS,
        "portable/pale-workspace.toml",
        "portable/resolution.lock.toml",
        "portable/universe.lock.toml",
    ),
    Path("integrations/vscode/tests/fixtures/cross-leaf.v1"): (
        "contract-diagnostic.json",
        "effective-profile-wire.json",
    ),
}

WORKSPACE_LANGUAGE_PROJECTIONS = {
    Path("crates/pale-application/tests/fixtures/workspace-language"): None,
    Path("sdk/rust/tests/fixtures/workspace-language"): None,
    Path("interfaces/lsp/tests/fixtures/workspace-language"): (
        "populated/pale-workspace.toml",
        "populated/module.toml",
        "populated/alpha.toml",
        "populated/beta.toml",
        "populated/resolution.lock.toml",
        "populated/universe.lock.toml",
        "unadopted/universe.lock.toml",
    ),
}

CONFORMANCE_SETS = (
    (
        Path("contracts/conformance/cross-leaf.v1"),
        "pale.cross-leaf-conformance-manifest.v1",
        CROSS_LEAF_PROJECTIONS,
    ),
    (
        Path("contracts/conformance/workspace-language.v1"),
        "pale.workspace-language-conformance-manifest.v1",
        WORKSPACE_LANGUAGE_PROJECTIONS,
    ),
)


def sha256(data: bytes) -> str:
    """Return the lowercase SHA-256 digest of one byte string."""
    return hashlib.sha256(data).hexdigest()


def inventory(directory: Path) -> list[dict[str, object]]:
    """Return the deterministic regular-file inventory below a directory."""
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"missing regular conformance directory: {directory}")
    rows: list[dict[str, object]] = []
    for path in sorted(directory.rglob("*")):
        if path.is_dir():
            continue
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"non-regular conformance member: {path}")
        relative = path.relative_to(directory).as_posix()
        if relative == MANIFEST_NAME:
            continue
        data = path.read_bytes()
        rows.append({"path": relative, "sha256": sha256(data), "size": len(data)})
    return rows


def aggregate(rows: list[dict[str, object]]) -> str:
    """Bind path, digest, and size for every ordered inventory member."""
    preimage = "".join(
        f'{row["path"]}\0{row["sha256"]}\0{row["size"]}\n' for row in rows
    ).encode()
    return sha256(preimage)


def expected_manifest(
    root: Path,
    canonical_root: Path,
    schema: str,
    projections_by_root: dict[Path, tuple[str, ...] | None],
) -> dict[str, object]:
    """Construct the root-owned manifest from current canonical bytes."""
    rows = inventory(root / canonical_root)
    member_paths = tuple(str(row["path"]) for row in rows)
    projections = []
    for projection_root, selected in projections_by_root.items():
        members = member_paths if selected is None else selected
        unknown = sorted(set(members) - set(member_paths))
        if unknown:
            raise ValueError(
                f"{projection_root}: unknown canonical members: {unknown}"
            )
        projections.append(
            {"root": projection_root.as_posix(), "members": list(members)}
        )
    return {
        "schema": schema,
        "version": 1,
        "aggregate_sha256": aggregate(rows),
        "member_count": len(rows),
        "members": rows,
        "projections": projections,
    }


def verify_projection(
    root: Path,
    canonical_rows: dict[str, dict[str, object]],
    projection: dict[str, object],
) -> None:
    """Verify one bounded physical projection against canonical bytes."""
    projection_root = root / str(projection["root"])
    selected = tuple(str(member) for member in projection["members"])
    observed = {str(row["path"]): row for row in inventory(projection_root)}
    if set(observed) != set(selected):
        raise ValueError(
            f"{projection_root}: projection inventory drift: "
            f"missing={sorted(set(selected) - set(observed))} "
            f"extra={sorted(set(observed) - set(selected))}"
        )
    for relative in selected:
        expected = canonical_rows[relative]
        actual = observed[relative]
        if actual != expected:
            raise ValueError(
                f"{projection_root / relative}: projection bytes drift"
            )


def sync_projection(
    root: Path, canonical_root: Path, projection: dict[str, object]
) -> None:
    """Write the selected canonical members without deleting other bytes."""
    canonical_root = root / canonical_root
    projection_root = root / str(projection["root"])
    if projection_root.is_symlink():
        raise ValueError(f"projection root must not be a symlink: {projection_root}")
    projection_root.mkdir(parents=True, exist_ok=True)
    for member in projection["members"]:
        relative = str(member)
        source = canonical_root / relative
        target = projection_root / relative
        if target.is_symlink():
            raise ValueError(f"projection member must not be a symlink: {target}")
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)


def main() -> None:
    """Write or verify the deterministic conformance manifest and projections."""
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parent.parent
    )
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve(strict=True)
    for canonical_root, schema, projections in CONFORMANCE_SETS:
        expected = expected_manifest(root, canonical_root, schema, projections)
        manifest_path = root / canonical_root / MANIFEST_NAME
        rendered = json.dumps(expected, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
        if args.write:
            manifest_path.write_text(rendered, encoding="utf-8")
            for projection in expected["projections"]:
                sync_projection(root, canonical_root, projection)
        elif (
            not manifest_path.is_file()
            or manifest_path.read_text(encoding="utf-8") != rendered
        ):
            raise SystemExit(f"stale conformance manifest: {manifest_path}")

        canonical_rows = {str(row["path"]): row for row in expected["members"]}
        for projection in expected["projections"]:
            verify_projection(root, canonical_rows, projection)


if __name__ == "__main__":
    main()
