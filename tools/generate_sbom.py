#!/usr/bin/env python3
"""Generate Pale's deterministic root-source SBOM and dependency notices.

The generator is deliberately offline. Cargo owns the inward Rust dependency
graph. SDKs and interfaces built outside that workspace need their own
dependency inventories even though they share the Pale repository. The release
manifest must cover the actual shipped artifacts and selected external systems.
Missing licence evidence is emitted as NOASSERTION and blocks distribution.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tomllib
import uuid
from pathlib import Path, PurePosixPath


SPDX_VERSION = "SPDX-2.3"
DATA_LICENSE = "CC0-1.0"
PLATFORM = "aarch64-apple-darwin"
DOCUMENT_SEED = uuid.UUID("3786692e-59ad-5d28-bf9d-90dc06d41024")


# Registry metadata records the SPDX declaration, while licences such as BSD
# also require their complete redistribution notice to travel with Pale.
# Hashing the retained upstream file makes that obligation reproducible rather
# than relying on an unverified prose reference.
RETAINED_RUST_NOTICES = {
    "comrak": (
        "THIRD_PARTY_LICENSES/COMRAK-COPYING",
        "27293e2353ce5d83e07331ffc423051908b062787d148193b8927d50d9873bd5",
    ),
}

BUNDLED_SQLITE = {
    "name": "SQLite",
    "version": "3.51.3",
    "licence": "LicenseRef-SQLite-Public-Domain",
    "licence_path": "THIRD_PARTY_LICENSES/SQLITE-PUBLIC-DOMAIN",
    "licence_sha256": "4fd8f740525f17fdb92a53799c2068d0ed4eada365c2590416a6824bf3764261",
    "copyright": "SQLite is in the public domain",
    "carrier": "libsqlite3-sys",
    "carrier_version": "0.37.0",
    "rusqlite_version": "0.39.0",
}

OPTIONAL_GRAPHBLAS = {
    "name": "SuiteSparse:GraphBLAS",
    "version": "10.3.1",
    "licence": "Apache-2.0 AND BSD-2-Clause AND BSD-3-Clause",
    "licence_path": "THIRD_PARTY_LICENSES/SUITESPARSE-GRAPHBLAS-LICENSE",
    "licence_sha256": "8ebd0741af56d62cb98b5f031fd44e8a41826bf00fb0e80de14702273c9122d9",
    "copyright": "NOASSERTION",
    "source": "https://github.com/DrTimothyAldenDavis/GraphBLAS/archive/refs/tags/v10.3.1.tar.gz",
    "source_sha256": "a3c4de775f47d9b448d0f548234a6c321c45f9f6a54e32c9e3a41b28df55cd0a",
    "carrier": "issundb-graphblas-sys",
    "carrier_version": "0.1.0-alpha.19",
    "wrapper": "issundb-graphblas",
    "wrapper_version": "0.1.0-alpha.14",
    "closure": "pale-kernel/graphblas on non-WebAssembly targets",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def spdx_licence_expression(raw: str) -> str:
    """Normalize the three known legacy Cargo slash spellings to SPDX syntax."""
    return {
        "Apache-2.0/MIT": "Apache-2.0 OR MIT",
        "MIT/Apache-2.0": "MIT OR Apache-2.0",
    }.get(raw, raw)


def run(*args: str, cwd: Path) -> str:
    completed = subprocess.run(
        args,
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout


def confined_relative(raw: str) -> str:
    path = PurePosixPath(raw)
    if path.is_absolute() or not path.parts or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError(f"unconfined relative path: {raw!r}")
    return path.as_posix()


def retained_rust_notice_rows(source_root: Path) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for crate, (raw_path, expected_sha) in sorted(RETAINED_RUST_NOTICES.items()):
        path = confined_relative(raw_path)
        notice = source_root / path
        if not notice.is_file() or notice.is_symlink():
            raise ValueError(f"missing regular retained Rust notice: {notice}")
        actual_sha = sha256(notice.read_bytes())
        if actual_sha != expected_sha:
            raise ValueError(
                f"retained Rust notice drift for {crate}: expected={expected_sha} actual={actual_sha}"
            )
        rows.append({"crate": crate, "path": path, "sha256": actual_sha})
    return rows


def tooling_asset_rows(source_root: Path) -> list[dict[str, str]]:
    """Verify non-Cargo source assets used only by development tooling."""
    metadata_path = source_root / "tools/rustdoc-math/vendor.json"
    if not metadata_path.is_file() or metadata_path.is_symlink():
        raise ValueError(f"missing regular tooling metadata: {metadata_path}")
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    required = {
        "name",
        "version",
        "source",
        "tarball_integrity",
        "tarball_sha256",
        "module_path",
        "module_sha256",
        "license",
        "license_path",
        "license_sha256",
    }
    if set(metadata) != required:
        raise ValueError(
            "tooling metadata fields drift: "
            f"missing={sorted(required - set(metadata))} extra={sorted(set(metadata) - required)}"
        )
    if not re.fullmatch(r"[0-9a-f]{64}", metadata["tarball_sha256"]):
        raise ValueError("tooling tarball SHA-256 is not canonical lowercase hex")
    if not str(metadata["tarball_integrity"]).startswith("sha512-"):
        raise ValueError("tooling tarball integrity must carry an exact SHA-512 coordinate")

    module_path = confined_relative(metadata["module_path"])
    licence_path = confined_relative(metadata["license_path"])
    for label, path, expected_sha in (
        ("module", module_path, metadata["module_sha256"]),
        ("licence", licence_path, metadata["license_sha256"]),
    ):
        file = source_root / path
        if not file.is_file() or file.is_symlink():
            raise ValueError(f"missing regular tooling {label}: {file}")
        actual_sha = sha256(file.read_bytes())
        if actual_sha != expected_sha:
            raise ValueError(
                f"tooling {label} drift for {metadata['name']}: "
                f"expected={expected_sha} actual={actual_sha}"
            )

    return [
        {
            **metadata,
            "module_path": module_path,
            "license_path": licence_path,
            "role": "offline Rustdoc TeX-to-MathML authoring tool",
        }
    ]


def cargo_metadata(source_root: Path, features: str | None = None) -> dict[str, object]:
    """Resolve one exact offline Cargo feature closure for the SBOM."""
    args = [
        "cargo",
        "metadata",
        "--locked",
        "--offline",
        "--filter-platform",
        PLATFORM,
        "--format-version",
        "1",
    ]
    if features is not None:
        args.extend(("--features", features))
    return json.loads(run(*args, cwd=source_root))


def cargo_rows(source_root: Path) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    closures = {
        "default": cargo_metadata(source_root),
        "graphblas": cargo_metadata(source_root, "pale-kernel/graphblas"),
        "benchmark-memory": cargo_metadata(
            source_root, "pale-adapter-control-storage/bench-memory"
        ),
    }
    metadata = cargo_metadata(
        source_root,
        "pale-kernel/graphblas,pale-adapter-control-storage/bench-memory",
    )
    lock = tomllib.loads((source_root / "Cargo.lock").read_text(encoding="utf-8"))
    checksums = {
        (item["name"], item["version"], item.get("source")): item.get("checksum")
        for item in lock["package"]
    }
    workspace_ids = set(metadata["workspace_members"])
    closure_nodes = {
        name: {node["id"]: node for node in closure["resolve"]["nodes"]}
        for name, closure in closures.items()
    }
    packages: list[dict[str, object]] = []
    package_by_id: dict[str, dict[str, object]] = {}
    for package in metadata["packages"]:
        workspace = package["id"] in workspace_ids
        source = package.get("source")
        checksum = checksums.get((package["name"], package["version"], source))
        row: dict[str, object] = {
            "cargo_id": package["id"],
            "name": package["name"],
            "version": package["version"],
            "licence": package.get("license") or "NOASSERTION",
            "workspace": workspace,
            "source": source,
            "checksum": checksum,
            "closures": sorted(
                name for name, nodes in closure_nodes.items() if package["id"] in nodes
            ),
            "features_by_closure": {
                name: sorted(nodes[package["id"]]["features"])
                for name, nodes in closure_nodes.items()
                if package["id"] in nodes
            },
        }
        row["spdx_licence"] = spdx_licence_expression(str(row["licence"]))
        packages.append(row)
        package_by_id[package["id"]] = row
    packages.sort(key=lambda row: (str(row["name"]), str(row["version"]), str(row["source"])))

    manifest_packages = {package["id"]: package for package in metadata["packages"]}
    optional_edges: set[tuple[str, str]] = set()
    for node in metadata["resolve"]["nodes"]:
        manifest = manifest_packages[node["id"]]
        dependencies = {
            (dependency.get("rename") or dependency["name"]).replace("-", "_"): dependency
            for dependency in manifest["dependencies"]
        }
        optional_edges.update(
            (node["id"], dependency["pkg"])
            for dependency in node["deps"]
            if dependencies.get(dependency["name"], {}).get("optional", False)
        )

    edge_closures: dict[tuple[str, str], set[str]] = {}
    for closure_name, closure in closures.items():
        for node in closure["resolve"]["nodes"]:
            for dependency in node["deps"]:
                edge_closures.setdefault((node["id"], dependency["pkg"]), set()).add(
                    closure_name
                )
    relationships: list[dict[str, object]] = []
    for (source_id, target_id), names in sorted(edge_closures.items()):
        source_row = package_by_id[source_id]
        target_row = package_by_id[target_id]
        relationships.append(
            {
                "source": f'{source_row["name"]}@{source_row["version"]}',
                "target": f'{target_row["name"]}@{target_row["version"]}',
                "optional": (source_id, target_id) in optional_edges,
                "closures": sorted(names),
            }
        )
    return packages, relationships


def bundled_native_rows(
    source_root: Path, cargo: list[dict[str, object]]
) -> list[dict[str, str]]:
    """Verify native source carried inside default and optional Cargo closures."""
    manifest = json.loads(
        (source_root / "contracts/manifests/pale-control-storage.v1.json").read_text(
            encoding="utf-8"
        )
    )
    sqlite = manifest["sqlite"]
    expected_dependency = {
        "library_version": BUNDLED_SQLITE["version"],
        "link_strategy": "bundled",
    }
    if {key: value for key, value in sqlite.items() if key != "required_cargo_features"} != expected_dependency:
        raise ValueError("control-storage manifest SQLite dependency drift")
    required_features = sqlite.get("required_cargo_features")
    if (
        not isinstance(required_features, list)
        or not all(isinstance(feature, str) and feature for feature in required_features)
        or required_features != sorted(set(required_features))
        or "bundled" not in required_features
    ):
        raise ValueError("control-storage manifest has invalid required SQLite features")

    def exact_package(name: str, version: str) -> dict[str, object]:
        matches = [row for row in cargo if row["name"] == name and row["version"] == version]
        if len(matches) != 1:
            raise ValueError(f"expected one resolved {name}@{version}, found {len(matches)}")
        return matches[0]

    rusqlite = exact_package("rusqlite", BUNDLED_SQLITE["rusqlite_version"])
    carrier = exact_package(BUNDLED_SQLITE["carrier"], BUNDLED_SQLITE["carrier_version"])
    resolved_features = set(rusqlite["features_by_closure"].get("default", []))
    if missing := set(required_features) - resolved_features:
        raise ValueError(f"resolved rusqlite lacks required features: {', '.join(sorted(missing))}")
    if "bundled" not in set(carrier["features_by_closure"].get("default", [])):
        raise ValueError("resolved libsqlite3-sys does not carry bundled SQLite")

    def retained_licence(row: dict[str, str]) -> str:
        licence_path = confined_relative(row["licence_path"])
        licence = source_root / licence_path
        if not licence.is_file() or licence.is_symlink():
            raise ValueError(f'missing regular native-source notice for {row["name"]}: {licence}')
        actual_sha = sha256(licence.read_bytes())
        if actual_sha != row["licence_sha256"]:
            raise ValueError(
                f'native-source notice drift for {row["name"]}: '
                f'expected={row["licence_sha256"]} actual={actual_sha}'
            )
        return licence_path

    graphblas_wrapper = exact_package(
        OPTIONAL_GRAPHBLAS["wrapper"], OPTIONAL_GRAPHBLAS["wrapper_version"]
    )
    graphblas_carrier = exact_package(
        OPTIONAL_GRAPHBLAS["carrier"], OPTIONAL_GRAPHBLAS["carrier_version"]
    )
    if graphblas_wrapper["closures"] != ["graphblas"]:
        raise ValueError("GraphBLAS safe wrapper escaped its optional feature closure")
    if graphblas_carrier["closures"] != ["graphblas"]:
        raise ValueError("GraphBLAS native carrier escaped its optional feature closure")

    sqlite_licence_path = retained_licence(BUNDLED_SQLITE)
    graphblas_licence_path = retained_licence(OPTIONAL_GRAPHBLAS)
    return [
        {
            **BUNDLED_SQLITE,
            "licence_path": sqlite_licence_path,
            "carrier_checksum": str(carrier["checksum"]),
            "source": "https://www.sqlite.org/",
            "source_sha256": "",
            "closure": "default control-storage closure",
            "custody": (
                f'bundled by {BUNDLED_SQLITE["carrier"]}@{BUNDLED_SQLITE["carrier_version"]}'
            ),
        },
        {
            **OPTIONAL_GRAPHBLAS,
            "licence_path": graphblas_licence_path,
            "carrier_checksum": str(graphblas_carrier["checksum"]),
            "wrapper_checksum": str(graphblas_wrapper["checksum"]),
            "custody": (
                f'{OPTIONAL_GRAPHBLAS["wrapper"]}@{OPTIONAL_GRAPHBLAS["wrapper_version"]} '
                f'uses {OPTIONAL_GRAPHBLAS["carrier"]}@{OPTIONAL_GRAPHBLAS["carrier_version"]}, '
                "whose checksum-verifying build downloads and contains the pinned native source"
            ),
        },
    ]


def spdx_token(kind: str, name: str, version: str) -> str:
    raw = f"{kind}-{name}-{version}"
    return "SPDXRef-" + re.sub(r"[^A-Za-z0-9.-]", "-", raw)


def build_spdx(
    source_root: Path,
    created: str,
    cargo: list[dict[str, object]],
    dependencies: list[dict[str, object]],
    bundled_native: list[dict[str, str]],
    tooling_assets: list[dict[str, str]],
) -> dict[str, object]:
    cargo_lock_sha = sha256((source_root / "Cargo.lock").read_bytes())
    seed_rows = [
        *(
            f'cargo\0{row["name"]}\0{row["version"]}\0{row["source"]}\0{row["checksum"]}'
            f'\0{",".join(row["closures"])}'
            for row in cargo
        ),
        *(
            f'native\0{row["name"]}\0{row["version"]}\0{row["licence"]}'
            f'\0{row["carrier_checksum"]}\0{row["source"]}\0{row["source_sha256"]}'
            for row in bundled_native
        ),
        *(
            f'tooling\0{row["name"]}\0{row["version"]}\0{row["source"]}'
            f'\0{row["tarball_sha256"]}\0{row["tarball_integrity"]}'
            f'\0{row["module_sha256"]}\0{row["license_sha256"]}'
            for row in tooling_assets
        ),
    ]
    document_uuid = uuid.uuid5(DOCUMENT_SEED, "\n".join(seed_rows))
    namespace = f"https://rawthoughts.in/pale/sbom/{document_uuid}"

    packages: list[dict[str, object]] = []
    id_by_cargo_key: dict[str, str] = {}
    for row in cargo:
        key = f'{row["name"]}@{row["version"]}'
        spdx_id = spdx_token("cargo", str(row["name"]), str(row["version"]))
        if spdx_id in id_by_cargo_key.values():
            suffix = sha256(str(row["source"]).encode())[:12]
            spdx_id = f"{spdx_id}-{suffix}"
        id_by_cargo_key[key] = spdx_id
        comment = "Pale workspace crate" if row["workspace"] else "Resolved Cargo dependency"
        comment += f'; closure={",".join(row["closures"])}'
        if str(row["source"]).startswith("git+"):
            comment += f'; Cargo source={row["source"]}'
        if row["spdx_licence"] != row["licence"]:
            comment += f'; Cargo declared legacy expression {row["licence"]!r}, normalized to SPDX syntax'
        package: dict[str, object] = {
            "SPDXID": spdx_id,
            "name": row["name"],
            "versionInfo": row["version"],
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": row["spdx_licence"],
            "copyrightText": "NOASSERTION",
            "primaryPackagePurpose": "LIBRARY",
            "comment": comment,
        }
        if row["checksum"]:
            package["checksums"] = [{"algorithm": "SHA256", "checksumValue": row["checksum"]}]
        packages.append(package)

    native_ids: dict[str, str] = {}
    for row in bundled_native:
        spdx_id = spdx_token("native", row["name"], row["version"])
        native_ids[row["name"]] = spdx_id
        packages.append(
            {
                "SPDXID": spdx_id,
                "name": row["name"],
                "versionInfo": row["version"],
                "downloadLocation": row["source"],
                "filesAnalyzed": False,
                "licenseConcluded": row["licence"],
                "licenseDeclared": row["licence"],
                "copyrightText": row["copyright"],
                "primaryPackagePurpose": "LIBRARY",
                "comment": (
                    f'{row["custody"]}; closure={row["closure"]}; '
                    f'carrier checksum={row["carrier_checksum"]}; notice={row["licence_path"]}; '
                    f'notice sha256={row["licence_sha256"]}; '
                    f'source sha256={row["source_sha256"] or "not separately pinned here"}'
                ),
            }
        )
        if row["source_sha256"]:
            packages[-1]["checksums"] = [
                {"algorithm": "SHA256", "checksumValue": row["source_sha256"]}
            ]

    tooling_ids: dict[str, str] = {}
    for row in tooling_assets:
        spdx_id = spdx_token("tooling", row["name"], row["version"])
        tooling_ids[row["name"]] = spdx_id
        packages.append(
            {
                "SPDXID": spdx_id,
                "name": row["name"],
                "versionInfo": row["version"],
                "downloadLocation": row["source"],
                "filesAnalyzed": False,
                "licenseConcluded": row["license"],
                "licenseDeclared": row["license"],
                "copyrightText": "NOASSERTION",
                "primaryPackagePurpose": "LIBRARY",
                "checksums": [
                    {"algorithm": "SHA256", "checksumValue": row["tarball_sha256"]}
                ],
                "comment": (
                    f'{row["role"]}; vendored asset={row["module_path"]}; '
                    f'module sha256={row["module_sha256"]}; '
                    f'archive integrity={row["tarball_integrity"]}; '
                    f'licence={row["license_path"]}; '
                    f'licence sha256={row["license_sha256"]}; '
                    "not linked into Pale binaries or loaded by generated documentation"
                ),
                "externalRefs": [
                    {
                        "referenceCategory": "PACKAGE-MANAGER",
                        "referenceType": "purl",
                        "referenceLocator": f'pkg:npm/{row["name"].lower()}@{row["version"]}',
                    }
                ],
            }
        )

    relationships_out: list[dict[str, str]] = []
    for row in cargo:
        key = f'{row["name"]}@{row["version"]}'
        if row["workspace"]:
            relationships_out.append(
                {
                    "spdxElementId": "SPDXRef-DOCUMENT",
                    "relationshipType": "DESCRIBES",
                    "relatedSpdxElement": id_by_cargo_key[key],
                }
            )
    for spdx_id in tooling_ids.values():
        relationships_out.append(
            {
                "spdxElementId": "SPDXRef-DOCUMENT",
                "relationshipType": "DESCRIBES",
                "relatedSpdxElement": spdx_id,
            }
        )
    for edge in dependencies:
        if edge["optional"]:
            relationship = {
                "spdxElementId": id_by_cargo_key[edge["target"]],
                "relationshipType": "OPTIONAL_DEPENDENCY_OF",
                "relatedSpdxElement": id_by_cargo_key[edge["source"]],
            }
        else:
            relationship = {
                "spdxElementId": id_by_cargo_key[edge["source"]],
                "relationshipType": "DEPENDS_ON",
                "relatedSpdxElement": id_by_cargo_key[edge["target"]],
            }
        relationship["comment"] = f'closures={",".join(edge["closures"])}'
        relationships_out.append(relationship)
    for row in bundled_native:
        relationships_out.append(
            {
                "spdxElementId": id_by_cargo_key[
                    f'{row["carrier"]}@{row["carrier_version"]}'
                ],
                "relationshipType": "CONTAINS",
                "relatedSpdxElement": native_ids[row["name"]],
            }
        )
    relationships_out.sort(key=lambda row: (row["spdxElementId"], row["relationshipType"], row["relatedSpdxElement"]))

    return {
        "spdxVersion": SPDX_VERSION,
        "dataLicense": DATA_LICENSE,
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "Pale source distribution SBOM",
        "documentNamespace": namespace,
        "creationInfo": {
            "created": created,
            "creators": ["Organization: RawThoughts Enterprises Private Limited", "Tool: tools/generate_sbom.py"],
        },
        "documentComment": (
            f"Offline Cargo metadata source inventory; platform={PLATFORM}; "
            f"Cargo.lock sha256={cargo_lock_sha}. This is not a binary SBOM, "
            "and a resolution entry does not prove inclusion in a compiled artifact. "
            "It excludes independently built Pale components and is not an aggregate release receipt. Separately owned native systems are not "
            "embedded components, and NOASSERTION licensing fails the distribution-release gate."
        ),
        "hasExtractedLicensingInfos": [
            {
                "licenseId": row["licence"],
                "extractedText": (source_root / row["licence_path"]).read_text(
                    encoding="utf-8"
                ),
                "name": f'{row["name"]} public-domain dedication and blessing',
                "seeAlsos": ["https://www.sqlite.org/copyright.html"],
            }
            for row in bundled_native
            if row["licence"].startswith("LicenseRef-")
        ],
        "packages": sorted(packages, key=lambda package: str(package["SPDXID"])),
        "relationships": relationships_out,
    }


def notices_markdown(
    created: str,
    cargo: list[dict[str, object]],
    retained_rust_notices: list[dict[str, str]],
    bundled_native: list[dict[str, str]],
    tooling_assets: list[dict[str, str]],
) -> str:
    lines = [
        "# Pale third-party notices and source inventory",
        "",
        f"Generated offline on `{created}` by `tools/generate_sbom.py`.",
        "",
        "Pale itself is owned by RawThoughts Enterprises Private Limited, authored and",
        "originated by Harsh Joshi, and distributed under `AGPL-3.0-or-later`. The",
        "entries below retain their native ownership and applicable licence. This file",
        "does not relicense a dependency, grant alternative commercial terms, or prove a",
        "binary/release closure. `NOASSERTION` is fail-closed release evidence.",
        "",
        "This inventory covers the root Cargo workspace, not every independently built",
        "SDK, interface or integration in this repository.",
        "The aggregate release must cover each shipped artifact's dependencies and",
        "licences. Separately owned native systems retain their own inventories.",
    ]
    lines.extend(
        [
            "",
            "## Rust workspace and resolved dependencies",
            "",
            "Resolved with `cargo metadata --locked --offline --filter-platform",
            f"{PLATFORM}`. The closure labels below identify the default, GraphBLAS,",
            "and memory-benchmark metadata resolutions. Registry checksums and Git revisions come from `Cargo.lock`.",
            "Cargo metadata can retain optional dependencies that a selected build does",
            "not compile. These rows inventory resolved source; they do not establish",
            "binary contents or runtime capabilities.",
            "",
            "| crate | version | declared licence | kind | closure | source identity |",
            "|---|---|---|---|---|---|",
        ]
    )
    for row in cargo:
        kind = "workspace" if row["workspace"] else "dependency"
        closure = ", ".join(row["closures"])
        checksum = f'`sha256:{row["checksum"]}`' if row["checksum"] else "—"
        if str(row["source"]).startswith("git+"):
            checksum = f'`{row["source"]}`'
        lines.append(
            f'| `{row["name"]}` | `{row["version"]}` | `{row["licence"]}` | '
            f'{kind} | {closure} | {checksum} |'
        )
    lines.extend(
        [
            "",
            "## Retained Rust redistribution notices",
            "",
            "| crate | complete upstream notice | sha256 |",
            "|---|---|---|",
        ]
    )
    for row in retained_rust_notices:
        lines.append(f'| `{row["crate"]}` | `{row["path"]}` | `{row["sha256"]}` |')
    lines.extend(
        [
            "",
            "## Vendored documentation tooling",
            "",
            "| tool | version | exact source and asset | retained licence | role |",
            "|---|---|---|---|---|",
        ]
    )
    for row in tooling_assets:
        lines.append(
            f'| `{row["name"]}` | `{row["version"]}` | '
            f'[{row["source"]}]({row["source"]}) tarball sha256 `{row["tarball_sha256"]}`; '
            f'`{row["module_path"]}` sha256 `{row["module_sha256"]}` | '
            f'`{row["license"]}` — `{row["license_path"]}` sha256 '
            f'`{row["license_sha256"]}` | {row["role"]}; absent from runtime and generated pages |'
        )
    lines.extend(
        [
            "",
            "## Bundled native source",
            "",
            "| source | version | carrier | status and retained evidence |",
            "|---|---|---|---|",
        ]
    )
    for row in bundled_native:
        source = f'[{row["source"]}]({row["source"]})'
        if row["source_sha256"]:
            source += f' sha256 `{row["source_sha256"]}`'
        lines.append(
            f'| `{row["name"]}` | `{row["version"]}` | `{row["carrier"]}@{row["carrier_version"]}` | '
            f'{source}; `{row["licence"]}` — `{row["licence_path"]}` sha256 '
            f'`{row["licence_sha256"]}`; carrier sha256 `{row["carrier_checksum"]}`; '
            f'closure `{row["closure"]}`; custody: {row["custody"]} |'
        )
    lines.extend(
        [
            "",
            "## Release boundary",
            "",
            "This root-source inventory is not a release authorization. It is not a",
            "complete distribution or source-to-binary SBOM, and no publication receipt is present.",
            "Any `NOASSERTION` licence evidence remains fail-closed for distribution.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=Path.cwd())
    parser.add_argument("--output-root", type=Path)
    parser.add_argument("--created", required=True, help="SPDX timestamp, for example 2026-08-11T00:00:00Z")
    args = parser.parse_args()

    source_root = args.source_root.resolve(strict=True)
    output_root = (args.output_root or source_root).resolve(strict=True)
    retained_rust_notices = retained_rust_notice_rows(source_root)
    tooling_assets = tooling_asset_rows(source_root)
    cargo, relationships = cargo_rows(source_root)
    bundled_native = bundled_native_rows(source_root, cargo)
    spdx = build_spdx(
        source_root,
        args.created,
        cargo,
        relationships,
        bundled_native,
        tooling_assets,
    )
    notices = notices_markdown(
        args.created,
        cargo,
        retained_rust_notices,
        bundled_native,
        tooling_assets,
    )

    outputs = {
        output_root / "SBOM.spdx.json": json.dumps(spdx, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        output_root / "THIRD_PARTY_NOTICES.md": notices,
    }
    for path, rendered in outputs.items():
        path.write_text(rendered, encoding="utf-8")


if __name__ == "__main__":
    main()
