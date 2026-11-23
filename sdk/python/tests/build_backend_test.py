"""Packaging failure/ownership tests; native generation is tested separately."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from wheel.wheelfile import WheelFile


_ROOT = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location("pale_build_backend", _ROOT / "build_backend.py")
assert _SPEC is not None and _SPEC.loader is not None
backend = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(backend)


class BuildBackendTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.output = self.root / "output"
        self.output.mkdir()

    def wheel(self, destination, *, editable=False, two_native=False, second_record=False):
        name = "pypale-0.0.1-cp39-abi3-linux_x86_64.whl"
        with WheelFile(str(Path(destination) / name), "w") as archive:
            archive.writestr("pypale-0.0.1.dist-info/WHEEL", "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: false\nTag: cp39-abi3-linux_x86_64\n")
            archive.writestr("pypale-0.0.1.dist-info/METADATA", "Metadata-Version: 2.1\nName: pypale\nVersion: 0.0.1\n")
            archive.writestr("pypale-0.0.1.dist-info/licenses/LICENSE", "Preserved test licence bytes\n")
            if second_record:
                archive.writestr("unexpected-0.0.1.dist-info/RECORD", "not the owning record\n")
            if editable:
                archive.writestr("pypale.pth", str(self.root / "python"))
            else:
                archive.writestr("pypale/_native.abi3.so", b"not executable: packaging fixture")
                archive.writestr("pypale/__init__.pyi", "from ._native import *\n")
                if two_native:
                    archive.writestr("pypale/_native.other.so", b"ambiguous fixture")
        return name

    @staticmethod
    def render(_native, output):
        # Explicit packaging fixture, not a second stub generator.
        output.write_text("class GeneratedFixture: ...\n")

    def test_wheel_delegates_arguments_and_changes_only_stub_and_record(self):
        settings = {"build-args": ["--profile", "release", "--locked"]}
        with patch.object(backend.maturin, "build_wheel", side_effect=lambda path, *_: self.wheel(path)) as build, patch.object(backend, "_render_stub", side_effect=self.render):
            name = backend.build_wheel(str(self.output), settings, "metadata-owner")
        delegated = build.call_args.args[1]
        self.assertNotIn("build-args", delegated)
        self.assertEqual(
            delegated["maturin.build-args"][:3],
            ["--profile", "release", "--locked"],
        )
        self.assertEqual(delegated["maturin.build-args"][-2], "--out")
        self.assertEqual(build.call_args.args[2], "metadata-owner")
        with WheelFile(str(self.output / name)) as archive:
            self.assertEqual(archive.read("pypale/_native.abi3.so"), b"not executable: packaging fixture")
            self.assertEqual(archive.read("pypale/_native.pyi"), b"class GeneratedFixture: ...\n")
            self.assertEqual(archive.read("pypale-0.0.1.dist-info/licenses/LICENSE"), b"Preserved test licence bytes\n")
            for member in archive.namelist():
                archive.read(member)

    def test_failed_generator_does_not_publish_intermediate_wheel(self):
        with patch.object(backend.maturin, "build_wheel", side_effect=lambda path, *_: self.wheel(path)), patch.object(backend, "_render_stub", side_effect=RuntimeError("generation rejected")):
            with self.assertRaisesRegex(RuntimeError, "generation rejected"):
                backend.build_wheel(str(self.output))
        self.assertEqual(list(self.output.iterdir()), [])

    def test_ambiguous_native_output_is_rejected_before_generation(self):
        with patch.object(backend.maturin, "build_wheel", side_effect=lambda path, *_: self.wheel(path, two_native=True)), patch.object(backend, "_render_stub") as render:
            with self.assertRaisesRegex(RuntimeError, "exactly one"):
                backend.build_wheel(str(self.output))
        render.assert_not_called()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_second_record_is_not_permitted_to_change(self):
        with patch.object(backend.maturin, "build_wheel", side_effect=lambda path, *_: self.wheel(path, second_record=True)), patch.object(backend, "_render_stub") as render:
            with self.assertRaisesRegex(RuntimeError, "exactly one RECORD"):
                backend.build_wheel(str(self.output))
        render.assert_not_called()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_unrelated_content_mutation_is_rejected(self):
        def corrupt(native, output):
            self.render(native, output)
            native.write_bytes(b"changed native bytes")
        with patch.object(backend.maturin, "build_wheel", side_effect=lambda path, *_: self.wheel(path)), patch.object(backend, "_render_stub", side_effect=corrupt):
            with self.assertRaisesRegex(RuntimeError, "outside"):
                backend.build_wheel(str(self.output))
        self.assertEqual(list(self.output.iterdir()), [])

    def test_editable_updates_only_exact_generated_stub_and_preserves_old_on_failure(self):
        package = self.root / "python/pypale"
        package.mkdir(parents=True)
        native = package / "_native.abi3.so"
        native.write_bytes(b"editable fixture")
        stub = package / "_native.pyi"
        stub.write_text("old fixture\n")
        with patch.object(backend, "_ROOT", self.root), patch.object(backend.maturin, "build_editable", side_effect=lambda path, *_: self.wheel(path, editable=True)), patch.object(backend, "_render_stub", side_effect=RuntimeError("generation rejected")):
            with self.assertRaises(RuntimeError):
                backend.build_editable(str(self.output))
        self.assertEqual(stub.read_text(), "old fixture\n")
        self.assertEqual(list(self.output.iterdir()), [])
        with patch.object(backend, "_ROOT", self.root), patch.object(backend.maturin, "build_editable", side_effect=lambda path, *_: self.wheel(path, editable=True)), patch.object(backend, "_render_stub", side_effect=self.render):
            backend.build_editable(str(self.output))
        self.assertEqual(stub.read_text(), "class GeneratedFixture: ...\n")
        self.assertEqual(native.read_bytes(), b"editable fixture")

    def test_symlink_native_is_not_followed(self):
        binary = self.root / "outside.so"
        binary.write_bytes(b"fixture")
        package = self.root / "package"
        package.mkdir()
        (package / "_native.abi3.so").symlink_to(binary)
        with self.assertRaises(RuntimeError):
            backend._native_file(package)

    def test_late_editable_failure_keeps_new_stub_without_publishing_a_wheel(self):
        package = self.root / "python/pypale"
        package.mkdir(parents=True)
        (package / "_native.abi3.so").write_bytes(b"editable fixture")
        stub = package / "_native.pyi"
        stub.write_text("old fixture\n")
        with patch.object(backend, "_ROOT", self.root), patch.object(backend.maturin, "build_editable", side_effect=lambda path, *_: self.wheel(path, editable=True)), patch.object(backend, "_render_stub", side_effect=self.render), patch.object(backend, "_publish_wheel", side_effect=OSError("publication failed")):
            with self.assertRaisesRegex(OSError, "publication failed"):
                backend.build_editable(str(self.output))
        self.assertEqual(stub.read_text(), "class GeneratedFixture: ...\n")
        self.assertEqual(list(self.output.iterdir()), [])

    def test_native_change_during_generation_is_rejected(self):
        native = self.root / "_native.abi3.so"
        native.write_bytes(b"before")
        output = self.root / "_native.pyi"

        def run(command, **_kwargs):
            if command[0] == "rustc":
                return SimpleNamespace(stdout="host: aarch64-apple-darwin\n")
            native.write_bytes(b"after")
            output.write_text("class Fixture: ...\n")
            return SimpleNamespace(returncode=0)

        with patch.object(backend.subprocess, "run", side_effect=run):
            with self.assertRaisesRegex(RuntimeError, "changed during"):
                backend._render_stub(native, output)

    def test_wrong_editable_source_is_rejected_before_generation(self):
        with patch.object(backend.maturin, "build_editable", side_effect=lambda path, *_: self.wheel(path, editable=True)), patch.object(backend, "_render_stub") as render:
            with self.assertRaisesRegex(RuntimeError, "does not point"):
                backend.build_editable(str(self.output))
        render.assert_not_called()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_unqualified_packaging_tool_version_is_rejected(self):
        with patch.object(backend.importlib.metadata, "version", return_value="other"):
            with self.assertRaisesRegex(RuntimeError, "required"):
                backend._check_tools()

    def test_caller_cannot_replace_private_maturin_output(self):
        for arguments in (("--out", "other"), ("--out=other",), ("-o", "other")):
            with self.subTest(arguments=arguments):
                with self.assertRaisesRegex(RuntimeError, "owns Maturin's wheel output"):
                    backend._maturin_settings({"build-args": arguments}, self.output)

    def test_generated_stub_rejects_numeric_collision_counters(self):
        stub = self.root / "_native.pyi"
        stub.write_text(
            "class ProfileInspectionContext: ...\n"
            "class ProfileInspectionContext2: ...\n"
        )
        with self.assertRaisesRegex(RuntimeError, "numeric collision counter"):
            backend._check_stub_declaration_names(stub)

        stub.write_text(
            "class AttributableAnswerV1: ...\n"
            "class AttributableAnswerV114: ...\n"
        )
        with self.assertRaisesRegex(RuntimeError, "AttributableAnswerV114"):
            backend._check_stub_declaration_names(stub)

    def test_generated_stub_preserves_meaningful_numeric_names(self):
        stub = self.root / "_native.pyi"
        stub.write_text("class Sha256DigestV1: ...\nclass Http2PolicyV1: ...\n")
        backend._check_stub_declaration_names(stub)

    def test_generated_stub_rejects_repeated_version_and_owner_names(self):
        stub = self.root / "_native.pyi"
        stub.write_text(
            "class TypedDigestV1: ...\n"
            "class TypedDigestV1TypedDigestCanonicalizationV1: ...\n"
        )
        with self.assertRaisesRegex(RuntimeError, "repeats a version segment"):
            backend._check_stub_declaration_names(stub)

        stub.write_text(
            "class ProviderBindingV1: ...\n"
            "class ProviderBindingV1ProviderCoordinates: ...\n"
        )
        with self.assertRaisesRegex(RuntimeError, "repeats its owning declaration"):
            backend._check_stub_declaration_names(stub)

    def test_metadata_requirements_and_sdist_are_maturin_owned(self):
        for name in (
            "build_sdist", "get_requires_for_build_sdist", "get_requires_for_build_wheel",
            "get_requires_for_build_editable", "prepare_metadata_for_build_wheel",
            "prepare_metadata_for_build_editable",
        ):
            self.assertIs(getattr(backend, name), getattr(backend.maturin, name))


if __name__ == "__main__":
    unittest.main()
