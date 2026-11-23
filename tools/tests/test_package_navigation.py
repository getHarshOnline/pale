"""The real formatter gates index writes and leaves original sources untouched."""

import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
import package_navigation


class PackageFormattingTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        family = self.root / "packages/example"
        family.mkdir(parents=True)
        self.body = family / "method.md"
        self.original = family / "original.md"
        self.original.write_text("Original paragraph\nkeeps its historical wrapping.\n")
        self.catalog = {
            "packages": [{"directory": "example", "manifest": {"package": {"id": "example"}}}],
            "resources": [{"package": "example", "source": "method.md", "body": None,
                           "text": "one live body"}],
        }

    def run_check(self, formatter):
        with patch.object(sys, "argv", ["package_navigation.py", "--root", str(self.root),
                                       "--formatter", formatter, "--write"]), \
                patch.object(package_navigation, "read_packages", return_value=self.catalog), \
                patch.object(package_navigation, "package_command") as command:
            try:
                package_navigation.main()
            finally:
                self.command = command

    def test_real_formatter_blocks_writes_then_accepts_formatted_body(self):
        formatter = os.environ.get("PALE_TEST_PRETTIER") or shutil.which("prettier")
        if not formatter:
            self.skipTest("install Prettier or set PALE_TEST_PRETTIER to exercise its actual process")
        self.body.write_text("Read the source\nbefore changing its meaning.\n")
        before = {path: path.read_bytes() for path in (self.body, self.original)}
        with self.assertRaises(SystemExit) as failure:
            self.run_check(formatter)
        self.assertEqual(failure.exception.code, 1)
        self.command.assert_not_called()
        self.assertEqual(before, {path: path.read_bytes() for path in before})

        self.body.write_text("Read the source before changing its meaning.\n")
        before[self.body] = self.body.read_bytes()
        self.run_check(formatter)
        self.command.assert_called_once_with(self.root, "check", "--write")
        self.assertEqual(before, {path: path.read_bytes() for path in before})

    def test_missing_formatter_cannot_pass_or_regenerate_indexes(self):
        self.body.write_text("Read the source.\n")
        with self.assertRaises(SystemExit) as failure:
            self.run_check(str(self.root / "missing-prettier"))
        self.assertEqual(failure.exception.code, 1)
        self.command.assert_not_called()

    def test_replaced_body_symlink_is_refused_before_formatter_launch(self):
        self.body.symlink_to(self.original)
        with patch.object(package_navigation.subprocess, "run") as launch:
            with self.assertRaisesRegex(ValueError, "confined regular file"):
                package_navigation.check_formatting(self.root, self.catalog, "prettier")
        launch.assert_not_called()


if __name__ == "__main__":
    unittest.main()
