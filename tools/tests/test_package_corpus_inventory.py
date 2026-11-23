"""Detached source delivery preserves exact dependencies and remains verifiable."""

import copy
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from package_corpus_inventory import inventory, select_packages, source_paths


class PackageSourceDelivery(unittest.TestCase):
    def test_design_source_closure_is_detached_and_self_verifying(self):
        selected = select_packages(inventory(ROOT), ["pale.design"])
        self.assertEqual([row["id"] for row in selected["packages"]],
                         ["pale.design", "pale.engineering", "pale.evaluation",
                          "pale.governance", "pale.model-work", "pale.research"])
        with tempfile.TemporaryDirectory(prefix="pale-package-source-") as directory:
            detached = Path(directory) / "source"
            detached.mkdir()
            expected = Path(directory) / "inventory.json"
            expected.write_text(json.dumps(selected))
            for relative in source_paths(selected):
                target = detached / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, target)
            environment = {key: value for key, value in os.environ.items() if key != "PYTHONPATH"}
            command = [sys.executable, str(detached / "tools/package_corpus_inventory.py"), str(detached),
                       "--verify", str(expected)]
            run = subprocess.run(command, cwd=detached, env=environment,
                                 text=True, capture_output=True, check=True)
            self.assertEqual(json.loads(run.stdout), selected)
            self.assertEqual({path.relative_to(detached).as_posix()
                              for path in detached.rglob("*") if path.is_file()},
                             set(source_paths(selected)))
            # An undeclared body cannot silently enter the source delivery.
            (detached / "packages/design/unowned.md").write_text("unselected\n")
            rejected = subprocess.run(command, cwd=detached, env=environment,
                                      text=True, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("membership differs", rejected.stderr)

    def test_unknown_selection_and_unrepresentable_archive_paths_are_refused(self):
        document = inventory(ROOT)
        with self.assertRaisesRegex(ValueError, "unknown Package"):
            select_packages(document, ["pale.missing"])
        changed = copy.deepcopy(document)
        changed["verification_sources"][0]["path"] = "tools/line\nbreak.py"
        with self.assertRaisesRegex(ValueError, "archive input list"):
            source_paths(changed)
        self.assertEqual(document, inventory(ROOT))


if __name__ == "__main__":
    unittest.main()
