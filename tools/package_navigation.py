#!/usr/bin/env python3
"""Check Package navigation and preview bodies through Pale’s shared reader."""

import argparse
from pathlib import Path
import subprocess

from package_reader import package_command, read_packages


def check_formatting(root, catalog, formatter):
    """Check live bodies with the repository formatter, leaving authored text unchanged."""
    root = root.resolve()
    directories = {entry["manifest"]["package"]["id"]: entry["directory"]
                   for entry in catalog["packages"]}
    paths = set()
    for resource in catalog["resources"]:
        if resource["text"] is None:
            continue
        path = root / "packages" / directories[resource["package"]] / (
            resource["body"] or resource["source"])
        if not path.is_file() or path.resolve() != path or not path.is_relative_to(root / "packages"):
            raise ValueError("Package body is no longer a confined regular file")
        paths.add(str(path))
    paths = sorted(paths)
    config = Path(__file__).resolve().parents[1] / ".prettierrc.json"
    failed = False
    for start in range(0, len(paths), 64):
        result = subprocess.run(
            [formatter, "--check", "--config", str(config), "--prose-wrap", "never",
             *paths[start:start + 64]],
            cwd=root, check=False,
        )
        failed |= result.returncode != 0
    if failed:
        raise ValueError("Package formatting failed; format the reported live bodies and rerun")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--formatter", default="prettier", metavar="EXECUTABLE",
                        help="Prettier executable for read-only live-body checks (default: prettier)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--body", help="show one body; resolve its links from its Markdown folder")
    args = parser.parse_args()
    try:
        if args.body:
            print(package_command(args.root, "show", args.body), end="")
        else:
            check_formatting(args.root, read_packages(args.root), args.formatter)
            package_command(args.root, "check", *(["--write"] if args.write else []))
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
