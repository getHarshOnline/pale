#!/usr/bin/env python3
"""Inventory declared Package sources and their notices without compiling them."""

import argparse
import json
from pathlib import Path
import sys

# Inventory is a read-only operation, including when run from a detached bundle.
sys.dont_write_bytecode = True
from generate_sbom import confined_relative, sha256
from package_reader import read_packages


UPSTREAM_NOTICES = (
    "NOTICE",
    "THIRD_PARTY_LICENSES/ECC-LICENSE",
    "THIRD_PARTY_LICENSES/PONYTAIL-LICENSE",
)


def regular_file(root, relative):
    path = root / confined_relative(relative)
    if not path.is_file() or path.resolve() != path:
        raise ValueError(f"missing or unconfined regular file: {relative}")
    return path


def file_record(root, relative, expected=None):
    payload = regular_file(root, relative).read_bytes()
    digest = sha256(payload)
    if expected is not None and digest != expected:
        raise ValueError(f"retained evidence drift: {relative}")
    return {"path": relative, "sha256": digest}


def package_rows(root):
    rows = []
    for entry in read_packages(root)["packages"]:
        manifest = root / "packages" / entry["directory"] / "package.toml"
        relative = manifest.relative_to(root).as_posix()
        document = entry["manifest"]
        package = document["package"]
        declared = {manifest}
        files = [file_record(root, relative)]
        for source in package["sources"]:
            path = manifest.parent / confined_relative(source["path"])
            if path in declared:
                raise ValueError(f"repeated Package source: {path.relative_to(root)}")
            declared.add(path)
            files.append(file_record(root, path.relative_to(root).as_posix()))
        members = list(manifest.parent.rglob("*"))
        if any(path.is_symlink() for path in members) or {
            path for path in members if path.is_file()
        } != declared:
            raise ValueError(f"Package membership differs from its manifest: {relative}")
        rows.append({
            "id": package["id"],
            "version": package["version"],
            "declared_license": document["license"]["spdx"],
            "dependencies": sorted(package["dependencies"], key=lambda row: row["id"]),
            "files": sorted(files, key=lambda row: row["path"]),
        })
    versions = {row["id"]: row["version"] for row in rows}
    if not rows or len(versions) != len(rows):
        raise ValueError("expected a nonempty corpus with unique Package IDs")
    for row in rows:
        for dependency in row["dependencies"]:
            if versions.get(dependency["id"]) != dependency["version"]:
                raise ValueError(f"unresolved Package pin in {row['id']}: {dependency['id']}")
    return sorted(rows, key=lambda row: row["id"])


def inventory(root):
    root = root.resolve()
    return {
        "scope": "declared Package sources and the notices that travel with them",
        "packages": package_rows(root),
        "corpus_notices": [file_record(root, f"packages/{name}")
                           for name in ("LICENSE", "NOTICE")] + [file_record(root, "CITATION.cff")]
                          + [file_record(root, name) for name in UPSTREAM_NOTICES],
        "verification_sources": [file_record(root, f"tools/{name}") for name in
                                 ("package_corpus_inventory.py", "package_reader.py", "generate_sbom.py")],
    }


def select_packages(document, requested):
    """Select source files by exact manifest dependency, not host/profile meaning."""
    if not requested:
        return document
    packages = {row["id"]: row for row in document["packages"]}
    selected = set()
    pending = list(requested)
    while pending:
        identity = pending.pop()
        if identity not in packages:
            raise ValueError(f"unknown Package source selection: {identity}")
        if identity in selected:
            continue
        selected.add(identity)
        pending.extend(row["id"] for row in packages[identity]["dependencies"])
    return {**document, "packages": [row for row in document["packages"]
                                    if row["id"] in selected]}


def source_paths(document):
    """Return the exact source/notice/verification closure for native archive tools."""
    records = [row for package in document["packages"] for row in package["files"]]
    for key in ("corpus_notices", "verification_sources"):
        records.extend(document[key])
    paths = sorted({row["path"] for row in records})
    if any(any(ord(char) < 32 or ord(char) == 127 for char in path) or
           confined_relative(path) != path or path.startswith("-") for path in paths):
        raise ValueError("source path cannot be represented in an archive input list")
    return paths


def verify_inventory(root, document):
    """Verify delivered bytes against the inventory produced by Pale's reader.

    A detached delivery needs Python and its saved inventory, not another Package
    parser or a Rust toolchain. Its trusted inventory supplies the expected set.
    """
    root = root.resolve()
    source_paths(document)  # Check every archive path before accessing any file.
    records = [file for package in document["packages"] for file in package["files"]]
    for name in ("corpus_notices", "verification_sources"):
        records.extend(document[name])
    for record in records:
        file_record(root, record["path"], record["sha256"])
    expected = set()
    directories = set()
    for package in document["packages"]:
        files = {row["path"] for row in package["files"]}
        manifests = [path for path in files if Path(path).name == "package.toml"]
        if len(manifests) != 1:
            raise ValueError("Each delivered Package needs one manifest")
        directory = Path(manifests[0]).parent
        if directory in directories or any(not Path(path).is_relative_to(directory) for path in files):
            raise ValueError("Package inventory repeats or escapes its source directory")
        directories.add(directory)
        expected.update(files)
    actual = set()
    for directory in directories:
        for path in (root / directory).rglob("*"):
            if path.is_symlink():
                raise ValueError("Package membership differs: linked source")
            if path.is_file():
                actual.add(path.relative_to(root).as_posix())
    observed_manifests = {p.parent.relative_to(root) for p in (root / "packages").rglob("package.toml")}
    if actual != expected or observed_manifests != directories:
        raise ValueError("Package membership differs from its saved inventory")
    return document


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="repository or detached Package source root")
    parser.add_argument("--package", action="append", default=[],
                        help="include this exact Package and its declared dependencies; repeatable")
    parser.add_argument("--paths", action="store_true",
                        help="emit relative source paths for a native archive tool, without writing files")
    parser.add_argument("--verify", type=Path,
                        help="verify detached bytes against a saved inventory, without a compiler")
    args = parser.parse_args()
    try:
        if args.verify:
            if args.package:
                raise ValueError("Verify the complete saved inventory; select Packages when creating it")
            document = verify_inventory(args.root, json.loads(args.verify.read_text()))
        else:
            document = select_packages(inventory(args.root), args.package)
        print("\n".join(source_paths(document)) if args.paths else json.dumps(document, indent=2))
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
