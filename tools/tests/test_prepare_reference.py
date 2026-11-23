"""Check source capture boundaries without substituting for native SDK builds."""

import argparse
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import prepare_reference as preparation


class PreparationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()

    def test_selection_includes_uncommitted_work_but_excludes_deleted_and_ignored_files(self):
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True)
        (self.root / "tracked").write_text("held work")
        (self.root / "deleted").write_text("old")
        subprocess.run(["git", "add", "tracked", "deleted"], cwd=self.root, check=True)
        (self.root / "deleted").unlink()
        (self.root / "untracked").write_text("new source")
        (self.root / ".gitignore").write_text("build/\n")
        (self.root / "build").mkdir()
        (self.root / "build" / "artifact").write_text("not source")
        self.assertEqual(preparation.selected_files(self.root), [".gitignore", "tracked", "untracked"])

    def test_capture_rejects_links_and_parent_escape(self):
        (self.root / "real").mkdir()
        (self.root / "real" / "file").write_text("data")
        (self.root / "linked").symlink_to(self.root / "real", target_is_directory=True)
        for relative in ("linked/file", "../outside", str(self.root / "real/file")):
            with self.subTest(relative=relative), self.assertRaises(ValueError):
                preparation.regular_bytes(self.root, relative)
        with self.assertRaises(ValueError):
            preparation.tree_files(self.root / "missing")

    def test_options_keep_exact_bytes_and_reject_tool_selections(self):
        args = argparse.Namespace()
        for argument, name, _ in preparation.OPTIONS:
            path = self.root / name
            path.write_bytes(b'{ "sourceChoice": true }\n')
            setattr(args, argument, str(path))
        self.assertTrue(all(value == b'{ "sourceChoice": true }\n'
                            for value in preparation.source_options(args).values()))
        for argument, _, forbidden in preparation.OPTIONS:
            path = Path(getattr(args, argument))
            for field in forbidden:
                path.write_text(json.dumps({field: "/machine/tool"}))
                with self.subTest(field=field), self.assertRaises(ValueError):
                    preparation.source_options(args)
            path.write_text("{}")

    def archive(self, members):
        archive = self.root / "contracts.tgz"
        with tarfile.open(archive, "w:gz") as output:
            for name, kind in members:
                member = tarfile.TarInfo(name)
                if kind == "link":
                    member.type = tarfile.SYMTYPE
                    member.linkname = "../../outside"
                    output.addfile(member)
                else:
                    member.size = 5
                    output.addfile(member, io.BytesIO(b"bytes"))
        return archive

    def test_packed_dependency_preserves_bytes(self):
        archive = self.archive([("package/dist/index.d.ts", "file")])
        destination = self.root / "dependency"
        preparation.unpack_contracts(archive, destination)
        self.assertEqual((destination / "dist/index.d.ts").read_bytes(), b"bytes")

    def test_archive_validation_precedes_extraction(self):
        for invalid in ([("package/../outside", "file")], [("package/link", "link")],
                        [("package/dist/file", "file"), ("package/dist/./file", "file")]):
            destination = self.root / "dependency"
            archive = self.archive([("package/good", "file"), *invalid])
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                preparation.unpack_contracts(archive, destination)
            self.assertFalse(destination.exists())

    def test_existing_output_is_never_replaced(self):
        output = self.root / "prepared"
        output.mkdir()
        (output / "original").write_text("keep")
        args = argparse.Namespace(source=str(self.root), output=str(output))
        with self.assertRaisesRegex(ValueError, "fresh directory"):
            preparation.prepare(args)
        self.assertEqual((output / "original").read_text(), "keep")

    def test_failed_native_command_retains_its_real_status_and_output(self):
        evidence = self.root / "evidence"
        evidence.mkdir()
        commands = preparation.Commands(self.root, evidence)
        with self.assertRaisesRegex(RuntimeError, "failed \\(7\\)"):
            commands.run("native-failure", ".", [sys.executable, "-c", "print('failure evidence'); raise SystemExit(7)"])
        self.assertEqual(commands.results[0]["exit_code"], 7)
        log = evidence / commands.results[0]["log"]
        self.assertEqual(log.read_text(), "failure evidence\n")
        self.assertEqual(commands.results[0]["log_sha256"], preparation.digest(log.read_bytes()))
        self.assertEqual(commands.environment["CARGO_TARGET_DIR"], str(self.root / "target"))
        self.assertNotIn("RUSTC_WRAPPER", commands.environment)

    def copied_source(self):
        source = self.root / "source"
        build = self.root / "build"
        source.mkdir()
        files = ["crates/example/src/lib.rs", "docs/workspace.md", "sdk/js/index.d.ts"]
        for name in files:
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("selected bytes\n")
            preparation.copy_member(source, build, name)
        return source, build, preparation.inventory(source, files)

    def test_initial_copy_must_match_even_for_generated_outputs(self):
        source, build, expected = self.copied_source()
        self.assertEqual(preparation.verify_copied_source(build, expected), {})
        (build / "sdk/js/index.d.ts").write_text("copy was altered")
        with self.assertRaisesRegex(ValueError, "copied source changed: sdk/js/index.d.ts"):
            preparation.verify_copied_source(build, expected)
        self.assertEqual(preparation.inventory(source, sorted(expected)), expected)

    def test_build_script_edits_to_rust_or_manual_are_rejected_while_live_source_is_unchanged(self):
        source, build, expected = self.copied_source()
        for name in ("crates/example/src/lib.rs", "docs/workspace.md"):
            with self.subTest(name=name):
                subprocess.run([sys.executable, "-c",
                                "from pathlib import Path; import sys; Path(sys.argv[1]).write_text('changed by build')",
                                name], cwd=build, check=True)
                with self.assertRaisesRegex(ValueError, "copied source changed"):
                    preparation.verify_copied_source(build, expected, preparation.GENERATED_SOURCE_FILES)
                self.assertEqual(preparation.inventory(source, sorted(expected)), expected)
                preparation.copy_member(source, build, name)
        (build / "sdk/js/index.d.ts").write_text("fresh generated declaration")
        self.assertEqual(set(preparation.verify_copied_source(
            build, expected, preparation.GENERATED_SOURCE_FILES)), {"sdk/js/index.d.ts"})
        (build / "docs/workspace.md").chmod(0o700)
        with self.assertRaisesRegex(ValueError, "copied source changed: docs/workspace.md"):
            preparation.verify_copied_source(build, expected, preparation.GENERATED_SOURCE_FILES)

    def test_missing_executable_retains_attempt_and_empty_log(self):
        evidence = self.root / "evidence"
        evidence.mkdir()
        commands = preparation.Commands(self.root, evidence)
        arguments = [str(self.root / "missing-executable"), "--version"]
        with self.assertRaises(FileNotFoundError):
            commands.run("missing", ".", arguments)
        attempt, = commands.results
        self.assertEqual(attempt["arguments"], arguments)
        self.assertIsNone(attempt["exit_code"])
        self.assertEqual(attempt["error"]["type"], "FileNotFoundError")
        self.assertEqual((evidence / attempt["log"]).read_bytes(), b"")
        self.assertEqual(attempt["log_sha256"], preparation.digest(b""))

    def test_interruption_preserves_attempt_and_partial_log(self):
        evidence = self.root / "evidence"
        evidence.mkdir()
        commands = preparation.Commands(self.root, evidence)

        def interrupt(*args, **kwargs):
            self.assertEqual(len(commands.results), 1)
            self.assertIsNone(commands.results[0]["exit_code"])
            kwargs["stdout"].write(b"partial native output\n")
            raise KeyboardInterrupt()

        with patch.object(preparation.subprocess, "run", side_effect=interrupt):
            with self.assertRaises(KeyboardInterrupt):
                commands.run("interrupted", ".", [sys.executable, "--version"])
        attempt, = commands.results
        self.assertIsNone(attempt["exit_code"])
        self.assertEqual(attempt["error"]["type"], "KeyboardInterrupt")
        data = (evidence / attempt["log"]).read_bytes()
        self.assertEqual(data, b"partial native output\n")
        self.assertEqual(attempt["log_sha256"], preparation.digest(data))


if __name__ == "__main__":
    unittest.main()
