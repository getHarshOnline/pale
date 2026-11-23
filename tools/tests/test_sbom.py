"""Check native dependency requirements against resolved feature evidence."""

import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest


REPOSITORY = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("sbom", REPOSITORY / "tools/generate_sbom.py")
assert SPEC is not None and SPEC.loader is not None
sbom = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sbom)


class NativeDependencyTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.manifest_path = self.root / "contracts/manifests/pale-control-storage.v1.json"
        self.manifest_path.parent.mkdir(parents=True)
        self.manifest = json.loads(
            (REPOSITORY / "contracts/manifests/pale-control-storage.v1.json").read_text()
        )
        self.write_manifest()
        for native in [sbom.BUNDLED_SQLITE, sbom.OPTIONAL_GRAPHBLAS]:
            destination = self.root / native["licence_path"]
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPOSITORY / native["licence_path"], destination)
        self.cargo = [
            {
                "name": "rusqlite",
                "version": sbom.BUNDLED_SQLITE["rusqlite_version"],
                "features_by_closure": {
                    "default": list(self.manifest["sqlite"]["required_cargo_features"])
                },
            },
            {
                "name": sbom.BUNDLED_SQLITE["carrier"],
                "version": sbom.BUNDLED_SQLITE["carrier_version"],
                "features_by_closure": {"default": ["bundled"]},
                "checksum": "a" * 64,
            },
            *[
                {
                    "name": sbom.OPTIONAL_GRAPHBLAS[kind],
                    "version": sbom.OPTIONAL_GRAPHBLAS[f"{kind}_version"],
                    "closures": ["graphblas"],
                    "checksum": "b" * 64,
                }
                for kind in ["wrapper", "carrier"]
            ],
        ]

    def write_manifest(self) -> None:
        self.manifest_path.write_text(json.dumps(self.manifest, indent=2) + "\n")

    def test_every_declared_feature_must_be_resolved(self) -> None:
        self.assertEqual(len(sbom.bundled_native_rows(self.root, self.cargo)), 2)
        for feature in self.manifest["sqlite"]["required_cargo_features"]:
            with self.subTest(feature=feature):
                cargo = copy.deepcopy(self.cargo)
                cargo[0]["features_by_closure"]["default"].remove(feature)
                with self.assertRaisesRegex(ValueError, f"lacks required features: {feature}"):
                    sbom.bundled_native_rows(self.root, cargo)

    def test_manifest_requirements_are_not_a_second_fixed_feature_list(self) -> None:
        self.manifest["sqlite"]["required_cargo_features"].append("trace")
        self.manifest["sqlite"]["required_cargo_features"].sort()
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "lacks required features: trace"):
            sbom.bundled_native_rows(self.root, self.cargo)
        self.cargo[0]["features_by_closure"]["default"].append("trace")
        self.assertEqual(len(sbom.bundled_native_rows(self.root, self.cargo)), 2)

    def test_native_carrier_and_optional_closure_evidence_remain_required(self) -> None:
        self.cargo[1]["features_by_closure"]["default"] = []
        with self.assertRaisesRegex(ValueError, "does not carry bundled SQLite"):
            sbom.bundled_native_rows(self.root, self.cargo)
        self.cargo[1]["features_by_closure"]["default"] = ["bundled"]
        self.cargo[2]["closures"] = ["default", "graphblas"]
        with self.assertRaisesRegex(ValueError, "escaped its optional feature closure"):
            sbom.bundled_native_rows(self.root, self.cargo)


class GitDependencyTests(unittest.TestCase):
    def test_outputs_keep_the_exact_git_source_without_inventing_a_checksum(self):
        source = "git+https://example.com/shared.git?rev=" + "a" * 40 + "#" + "a" * 40
        cargo = [{
            "name": "shared-files", "version": "0.0.1", "source": source,
            "checksum": None, "closures": ["default"], "workspace": False,
            "licence": "AGPL-3.0-or-later", "spdx_licence": "AGPL-3.0-or-later",
        }]
        document = sbom.build_spdx(REPOSITORY, "2026-11-15T00:00:00Z", cargo, [], [], [])
        package = document["packages"][0]
        self.assertIn(source, package["comment"])
        self.assertNotIn("checksums", package)
        notices = sbom.notices_markdown("2026-11-15T00:00:00Z", cargo, [], [], [])
        self.assertIn(source, notices)


if __name__ == "__main__":
    unittest.main()
