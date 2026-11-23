"""Characterize absolute calls through the installed Python binding."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import platform
import time
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

import pypale


SMOKE_SAMPLES = 5
CHARACTERIZATION_SAMPLES = 201
MAX_COORDINATE_BYTES = 512


@dataclass(frozen=True)
class Configuration:
    """Caller-bound run identity and sample count."""

    smoke: bool
    samples: int
    source: str
    run: str
    hardware: str


@dataclass(frozen=True)
class Case:
    """One measured operation and its conservative byte lower bounds."""

    identifier: str
    request_bytes: int
    response_bytes: int
    operation: Callable[[], Any]


def coordinate(name: str, smoke: bool, fallback: str) -> str:
    """Resolve one bounded TSV-safe coordinate."""

    value = os.environ.get(name, fallback if smoke else "")
    if not value:
        raise ValueError(f"{name} is required for a characterized run")
    if len(value.encode()) > MAX_COORDINATE_BYTES:
        raise ValueError(f"{name} exceeds {MAX_COORDINATE_BYTES} bytes")
    if any(character in value for character in "\t\r\n"):
        raise ValueError(f"{name} must be one TSV-safe line")
    return value


def percentile(sorted_values: list[int], rank: int) -> int:
    """Return the repository-standard nearest-rank percentile."""

    index = ((rank * len(sorted_values) + 99) // 100) - 1
    return sorted_values[index]


def measure(samples: int, operation: Callable[[], Any]) -> tuple[int, int, str, int]:
    """Warm one call site, then observe absolute elapsed nanoseconds."""

    for _ in range(3):
        operation()
    observations = []
    for _ in range(samples):
        started = time.perf_counter_ns()
        operation()
        observations.append(time.perf_counter_ns() - started)
    observations.sort()
    p99 = str(percentile(observations, 99)) if samples >= 101 else ""
    return (
        percentile(observations, 50),
        percentile(observations, 95),
        p99,
        observations[-1],
    )


def fixture_digest(fixture_root: pathlib.Path, portable: dict[str, Any]) -> str:
    """Bind the report to the exact shared input members."""

    members = [
        "bundle-identity.json",
        "contract-diagnostic.json",
        "portable-journey.json",
        *(f"portable/{label}" for label in portable["document_labels"]),
    ]
    digest = hashlib.sha256(b"pale.cross-leaf-ffi-benchmark-fixture.v1\0")
    for member in members:
        digest.update(member.encode())
        digest.update(b"\0")
        digest.update((fixture_root / member).read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def prove_laws(
    api: pypale.Pale,
    fixture: dict[str, Any],
    contract: dict[str, Any],
    portable: dict[str, Any],
    documents: list[tuple[str, bytes]],
) -> tuple[Any, bytes]:
    """Prove exact fixture equivalence before opening a timing interval."""

    identity = api.contract_bundle_identity()
    assert identity.member_count == fixture["member_count"]
    assert identity.schema_manifest_sha256 == fixture["schema_manifest_sha256"]
    assert identity.schema_set_sha256 == fixture["schema_set_sha256"]

    diagnostic_bytes = contract["canonical_diagnostic"].encode()
    assert api.admit_diagnostic(diagnostic_bytes) == diagnostic_bytes
    validated = api.validate_contract_document(
        contract["schema"]["registry_id"], diagnostic_bytes
    )
    assert validated.schema_id == contract["schema"]["registry_id"]
    assert validated.schema_sha256 == contract["schema"]["schema_sha256"]
    assert validated.canonical_bytes == diagnostic_bytes

    prior = api.compile_portable_workspace(portable["root_manifest"], documents)
    current = api.compile_portable_workspace(portable["root_manifest"], documents)
    actual_compilation = {
        key: getattr(prior, key) for key in portable["compilation"]
    }
    assert actual_compilation == portable["compilation"]

    comparison = api.compare_portable_workspaces(prior, current)
    page = comparison.impacts(0, 1)
    assert page.next_offset == portable["first_impact"]["next_offset"]
    assert len(page.entries) == 1
    impact = page.entries[0]
    subject = impact.subject()
    assert subject.kind == pypale.WorkspaceSubjectKind.Source
    assert subject.source_id == portable["first_impact"]["source_id"]
    assert impact.state == pypale.WorkspaceImpactState.Unaffected
    assert (
        impact.contributing_root_count
        == portable["first_impact"]["contributing_root_count"]
    )
    return comparison, diagnostic_bytes


def render_report(
    configuration: Configuration, fixture_sha256: str, cases: list[Case]
) -> str:
    """Render and checksum one stable TSV payload."""

    columns = [
        "source_coordinate",
        "fixture_sha256",
        "run_coordinate",
        "evidence_class",
        "leaf",
        "operation",
        "law_gate_passed",
        "request_bytes",
        "response_bytes",
        "copy_lower_bound_bytes",
        "samples",
        "p50_ns",
        "p95_ns",
        "p99_ns",
        "max_ns",
        "hardware",
        "runtime",
        "arch",
        "proof_ceiling",
        "release_evidence",
    ]
    rows = []
    for case in cases:
        p50, p95, p99, maximum = measure(
            configuration.samples, case.operation
        )
        rows.append(
            [
                configuration.source,
                fixture_sha256,
                configuration.run,
                "smoke_validation"
                if configuration.smoke
                else "local_characterization",
                "python",
                case.identifier,
                "true",
                str(case.request_bytes),
                str(case.response_bytes),
                str(case.request_bytes + case.response_bytes),
                str(configuration.samples),
                str(p50),
                str(p95),
                p99,
                str(maximum),
                configuration.hardware,
                platform.python_version(),
                f"{platform.system().lower()}-{platform.machine()}",
                "local_leaf_latency_and_payload_lower_bound_only",
                "false",
            ]
        )
    payload = "\t".join(columns) + "\n"
    payload += "".join("\t".join(row) + "\n" for row in rows)
    seal = hashlib.sha256(payload.encode()).hexdigest()
    return (
        payload
        + "# pale-native-ffi-benchmark-seal"
        + f"\trows={len(rows)}"
        + f"\tpayload_sha256={seal}"
        + f"\tsource_coordinate={configuration.source}"
        + f"\trun_coordinate={configuration.run}\n"
    )


def main() -> None:
    """Load the shared fixture, prove laws, and emit one report."""

    parser = argparse.ArgumentParser()
    parser.add_argument("fixture_root", type=pathlib.Path)
    parser.add_argument("--smoke", action="store_true")
    arguments = parser.parse_args()
    fixture_root = arguments.fixture_root.resolve()
    fixture = json.loads((fixture_root / "bundle-identity.json").read_text())
    contract = json.loads((fixture_root / "contract-diagnostic.json").read_text())
    portable = json.loads((fixture_root / "portable-journey.json").read_text())
    documents = [
        (label, (fixture_root / "portable" / label).read_bytes())
        for label in portable["document_labels"]
    ]
    api = pypale.Pale()
    comparison, diagnostic_bytes = prove_laws(
        api, fixture, contract, portable, documents
    )

    request_bytes = len(portable["root_manifest"].encode()) + sum(
        len(label.encode()) + len(document) for label, document in documents
    )
    response_bytes = (
        len(portable["compilation"]["workspace_id"].encode())
        + (3 * 64)
        + (7 * 4)
    )
    impact_response_bytes = (
        len(portable["first_impact"]["source_id"].encode())
        + len(portable["first_impact"]["state"].encode())
        + 8
    )
    cases = [
        Case(
            "bundle_identity",
            0,
            4 + (2 * 64),
            lambda: api.contract_bundle_identity().member_count,
        ),
        Case(
            "diagnostic_admission",
            len(diagnostic_bytes),
            len(diagnostic_bytes),
            lambda: len(api.admit_diagnostic(diagnostic_bytes)),
        ),
        Case(
            "contract_validation",
            len(contract["schema"]["registry_id"].encode())
            + len(diagnostic_bytes),
            len(diagnostic_bytes),
            lambda: len(
                api.validate_contract_document(
                    contract["schema"]["registry_id"], diagnostic_bytes
                ).canonical_bytes
            ),
        ),
        Case(
            "portable_compilation",
            request_bytes,
            response_bytes,
            lambda: api.compile_portable_workspace(
                portable["root_manifest"], documents
            ).source_document_count,
        ),
        Case(
            "retained_impact_page",
            14,
            impact_response_bytes,
            lambda: len(comparison.impacts(0, 1).entries),
        ),
    ]
    configuration = Configuration(
        smoke=arguments.smoke,
        samples=SMOKE_SAMPLES if arguments.smoke else CHARACTERIZATION_SAMPLES,
        source=coordinate(
            "PALE_FFI_BENCH_SOURCE_COORDINATE",
            arguments.smoke,
            "smoke-unsealed-worktree",
        ),
        run=coordinate(
            "PALE_FFI_BENCH_RUN_COORDINATE",
            arguments.smoke,
            "smoke-validation",
        ),
        hardware=coordinate(
            "PALE_FFI_BENCH_HARDWARE",
            arguments.smoke,
            "unspecified-smoke-host",
        ),
    )
    print(
        render_report(configuration, fixture_digest(fixture_root, portable), cases),
        end="",
    )


if __name__ == "__main__":
    main()
