"""Behavioral laws for bounded canonical fixture projections."""

import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest


REPOSITORY = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "constellation", REPOSITORY / "tools/constellation.py"
)
assert SPEC is not None and SPEC.loader is not None
constellation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(constellation)


class ConstellationTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.canonical = Path("contracts/conformance/cross-leaf.v1")
        self.destination = Path("crates/pale-application/tests/fixtures/cross-leaf.v1")
        shutil.copytree(REPOSITORY / self.canonical, self.root / self.canonical)
        self.manifest = constellation.expected_manifest(
            self.root,
            self.canonical,
            "pale.cross-leaf-conformance-manifest.v1",
            {self.destination: constellation.CROSS_LEAF_PROJECTIONS[self.destination]},
        )
        self.projection = self.manifest["projections"][0]
        self.rows = {row["path"]: row for row in self.manifest["members"]}
        constellation.sync_projection(self.root, self.canonical, self.projection)

    def verify(self) -> None:
        constellation.verify_projection(self.root, self.rows, self.projection)

    def member(self) -> Path:
        return self.root / self.destination / self.projection["members"][0]

    def test_projection_preserves_exact_selected_bytes_and_is_idempotent(self) -> None:
        self.verify()
        first = constellation.inventory(self.root / self.destination)
        self.assertEqual(
            first,
            sorted(
                (self.rows[member] for member in self.projection["members"]),
                key=lambda row: row["path"],
            ),
        )
        constellation.sync_projection(self.root, self.canonical, self.projection)
        self.assertEqual(first, constellation.inventory(self.root / self.destination))
        self.verify()

    def test_changed_member_is_rejected_and_sync_restores_canonical_bytes(self) -> None:
        self.member().write_bytes(b"changed fixture bytes")
        with self.assertRaisesRegex(ValueError, "projection bytes drift"):
            self.verify()
        constellation.sync_projection(self.root, self.canonical, self.projection)
        self.verify()

    def test_missing_member_is_rejected(self) -> None:
        self.member().unlink()
        with self.assertRaisesRegex(ValueError, "projection inventory drift"):
            self.verify()

    def test_extra_member_is_rejected_without_silent_deletion(self) -> None:
        extra = self.root / self.destination / "unexpected.toml"
        extra.write_bytes(b"owner = 'unselected'\n")
        constellation.sync_projection(self.root, self.canonical, self.projection)
        with self.assertRaisesRegex(ValueError, "projection inventory drift"):
            self.verify()
        self.assertEqual(extra.read_bytes(), b"owner = 'unselected'\n")

    def test_unknown_selected_member_cannot_form_a_manifest(self) -> None:
        with self.assertRaisesRegex(ValueError, "unknown canonical members"):
            constellation.expected_manifest(
                self.root,
                self.canonical,
                "pale.cross-leaf-conformance-manifest.v1",
                {self.destination: ("not-a-canonical-member.toml",)},
            )

    def test_workspace_navigation_projects_shared_locks_without_other_sources(self) -> None:
        canonical = Path("contracts/conformance/workspace-language.v1")
        shutil.copytree(REPOSITORY / canonical, self.root / canonical)
        manifest = constellation.expected_manifest(
            self.root,
            canonical,
            "pale.workspace-language-conformance-manifest.v1",
            constellation.WORKSPACE_LANGUAGE_PROJECTIONS,
        )
        rows = {row["path"]: row for row in manifest["members"]}
        projections = {row["root"]: row for row in manifest["projections"]}
        for projection in projections.values():
            constellation.sync_projection(self.root, canonical, projection)
            constellation.verify_projection(self.root, rows, projection)

        lock = "unadopted/universe.lock.toml"
        expected = (self.root / canonical / lock).read_bytes()
        sdk = Path("sdk/rust/tests/fixtures/workspace-language")
        lsp = Path("interfaces/lsp/tests/fixtures/workspace-language")
        self.assertEqual((self.root / sdk / lock).read_bytes(), expected)
        self.assertEqual((self.root / lsp / lock).read_bytes(), expected)
        self.assertEqual(len(constellation.inventory(self.root / lsp)), 7)
        self.assertFalse((self.root / lsp / "populated/alpha-handwritten.toml").exists())

        projection = projections[lsp.as_posix()]
        (self.root / lsp / lock).write_bytes(b"changed adoption")
        with self.assertRaisesRegex(ValueError, "projection bytes drift"):
            constellation.verify_projection(self.root, rows, projection)
        self.assertEqual((self.root / sdk / lock).read_bytes(), expected)
        constellation.sync_projection(self.root, canonical, projection)
        constellation.verify_projection(self.root, rows, projection)

        extra = self.root / lsp / "unregistered.toml"
        extra.write_bytes(b"unregistered source")
        constellation.sync_projection(self.root, canonical, projection)
        with self.assertRaisesRegex(ValueError, "projection inventory drift"):
            constellation.verify_projection(self.root, rows, projection)
        self.assertEqual(extra.read_bytes(), b"unregistered source")


if __name__ == "__main__":
    unittest.main()
