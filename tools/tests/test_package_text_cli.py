"""Writing measurements guide a reader without rejecting a substantive rewrite."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


TOOL = Path(__file__).resolve().parents[1] / "package_text.py"


class PackageTextCliTests(unittest.TestCase):
    def test_wording_comparison_reports_changes_without_failing_the_rewrite(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            before, after = root / "before", root / "after"
            for directory in (before, after):
                (directory / "packages").mkdir(parents=True)
            (before / "packages" / "review.md").write_text("# Review\n\nCheck the change.\n")
            (after / "packages" / "review.md").write_text(
                "# Review\n\nCompare Widgets and gadget.\n\nUse `gadget`.\n\n```\nwidget\n```\n"
            )
            vocabulary = root / "words.txt"
            vocabulary.write_text("widget*\n\ngadget\n", encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(TOOL), "--root", str(after), "--baseline", str(before),
                 "--word-list", str(vocabulary)],
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertTrue(report["word_list_configured"])
            self.assertEqual(report["comparison_warnings"], ["packages/review.md"])
            self.assertEqual(report["files"]["packages/review.md"]["flagged"], 2)
            self.assertTrue(report["files"]["packages/review.md"]["dense_sentences"])

    def test_no_list_reports_sizes_without_assuming_a_vocabulary(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "packages").mkdir()
            (root / "packages" / "review.md").write_text("Selected authority boundary.\n")
            result = subprocess.run(
                [sys.executable, str(TOOL), "--root", str(root), "--warn-words", "1"],
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertFalse(report["word_list_configured"])
            self.assertEqual(report["files"]["packages/review.md"]["words"], 3)
            self.assertEqual(report["files"]["packages/review.md"]["flagged"], 0)
            self.assertEqual(report["size_warnings"], ["packages/review.md"])

    def test_bad_or_missing_lists_fail_without_a_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            vocabulary = root / "words.txt"
            for contents in ("", "\n", "widget|gadget\n", "*\n", "two words\n"):
                with self.subTest(contents=contents):
                    vocabulary.write_text(contents, encoding="utf-8")
                    result = subprocess.run(
                        [sys.executable, str(TOOL), "--root", str(root), "--word-list", str(vocabulary)],
                        text=True, capture_output=True, check=False,
                    )
                    self.assertEqual(result.returncode, 2)
                    self.assertEqual(result.stdout, "")
                    self.assertIn("Word list", result.stderr)
            for arguments in (["--baseline", str(root)], ["--word-list", str(root / "missing.txt")],
                              ["--port-map", "map.json", "--word-list", str(vocabulary)]):
                with self.subTest(arguments=arguments):
                    result = subprocess.run(
                        [sys.executable, str(TOOL), "--root", str(root), *arguments],
                        text=True, capture_output=True, check=False,
                    )
                    self.assertEqual(result.returncode, 2)
                    self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
