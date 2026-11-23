"""Check the vault consumer's native-result boundary; Rust owns tree tests."""

from contextlib import redirect_stdout
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import build_sizes
import vault


class BuildSizeTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.build = self.root / "target"
        self.executable = self.root / "pale"

    def native(self, **changes):
        report = dict(root=str(self.build), logical_bytes=5, file_count=1,
                      skipped_symlinks=2, complete=True, errors=[])
        report.update(changes)
        # Doctor exits 1 when unrelated deployment evidence is unavailable.
        return subprocess.CompletedProcess([], 1, json.dumps({
            "completion": "complete", "result": {"checks_passed": False, "builds": [report]}
        }), "")

    def test_passes_explicit_selection_and_preserves_independent_result(self):
        with patch.object(build_sizes.subprocess, "run", return_value=self.native()) as run:
            report = build_sizes.measure(self.build, self.executable)
        self.assertTrue(report.complete)
        self.assertEqual((report.logical_bytes, report.file_count, report.skipped_symlinks), (5, 1, 2))
        self.assertEqual(run.call_args.args[0], [str(self.executable), "--result-format", "json", "doctor", "--build-root", str(self.build)])
        self.assertFalse(run.call_args.kwargs.get("shell", False))
        self.assertEqual(run.call_args.kwargs["timeout"], 120)

    def test_never_guesses_or_builds_an_executable(self):
        with patch.object(build_sizes.subprocess, "run") as run:
            for executable in (None, Path("pale")):
                report = build_sizes.measure(self.build, executable)
                self.assertFalse(report.complete)
                self.assertIsNone(report.logical_bytes)
                self.assertIn("--pale-executable", report.errors[0])
            run.assert_not_called()

    def test_preserves_partial_totals_and_native_errors(self):
        with patch.object(build_sizes.subprocess, "run", return_value=self.native(complete=False, errors=["file changed"])):
            report = build_sizes.measure(self.build, self.executable)
        self.assertFalse(report.complete)
        self.assertEqual(report.logical_bytes, 5)
        findings = vault.build_findings(report, 4)
        self.assertEqual({finding.code for finding in findings}, {"build-read-error", "build-size"})
        self.assertIn("at least 5", findings[-1].message)

    def test_unavailable_native_process_never_becomes_empty_tree(self):
        for failure in (FileNotFoundError("missing executable"), subprocess.TimeoutExpired("pale", 120)):
            with self.subTest(failure=failure), patch.object(build_sizes.subprocess, "run", side_effect=failure):
                report = build_sizes.measure(self.build, self.executable)
                self.assertFalse(report.complete)
                self.assertIsNone(report.logical_bytes)
                self.assertTrue(report.errors)

    def test_rejects_unrelated_or_inconsistent_output(self):
        replies = [self.native(root="/another-tree"), self.native(logical_bytes=None),
                   self.native(logical_bytes=-1), self.native(file_count=True),
                   self.native(complete=False), self.native(errors=["failed"]),
                   subprocess.CompletedProcess([], 4, "{}", "bad input"),
                   subprocess.CompletedProcess([], 0, "not JSON", ""),
                   subprocess.CompletedProcess([], 0, "[]", ""),
                   subprocess.CompletedProcess([], 0, "null", "")]
        for reply in replies:
            with self.subTest(reply=reply), patch.object(build_sizes.subprocess, "run", return_value=reply):
                report = build_sizes.measure(self.build, self.executable)
                self.assertFalse(report.complete)
                self.assertIsNone(report.logical_bytes)

    def test_vault_cli_projects_totals_and_inclusive_limits(self):
        for limit, expected_status in ((None, 0), (5, 0), (4, 1)):
            options = [] if limit is None else ["--build-limit", str(limit)]
            output = io.StringIO()
            with redirect_stdout(output), patch.object(build_sizes.subprocess, "run", return_value=self.native()):
                status = vault.main(["--root", str(self.root), "--pale-executable", str(self.executable),
                                     "--build-root", str(self.build), "--json", *options, "builds"])
            self.assertEqual(status, expected_status)
            report = json.loads(output.getvalue())
            self.assertEqual(report["builds"][0]["logical_bytes"], 5)
            self.assertEqual(report["changes"], [])

    def test_vault_check_reports_unavailable_native_inspection(self):
        policy = vault.Policy((self.root,), build_roots=(self.build,))
        self.assertIn("build-read-error", {finding.code for finding in vault.check(policy, suggest_moves=False)})


if __name__ == "__main__":
    unittest.main()
