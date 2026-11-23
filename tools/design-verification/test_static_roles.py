"""Regressions for interface values versus explicitly selected authored scene code."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("static_audit.py")


class SourceRoleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.scene = self.root / "scene.js"
        self.scene.write_text('animate(subject, {duration: 0.75, ease: "power2.inOut"});')

    def audit(self, *options):
        return subprocess.run([sys.executable, str(SCRIPT), "--token-roles", *map(str, options)],
                              capture_output=True, text=True)

    def test_shared_interface_timing_still_requires_tokens(self):
        result = self.audit(self.scene)
        self.assertEqual(result.returncode, 1)
        self.assertIn("literal-motion", result.stdout)

    def test_selected_scene_timing_is_reported_and_semantics_still_checked(self):
        result = self.audit("--authored-source", self.scene, self.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("TOKEN ROLE CHECKS EXCLUDED", result.stdout)
        (self.root / "host.html").write_text('<a href="#">Missing destination</a>')
        result = self.audit("--authored-source", self.scene, self.root)
        self.assertEqual(result.returncode, 1)
        self.assertIn("placeholder-link", result.stdout)

    def test_mixed_file_keeps_interface_timing_in_scope(self):
        interface = self.root / "controls.js"
        interface.write_text('animate(button, {duration: 0.2, ease: "linear"});')
        scene_source = self.scene.read_text()
        self.scene.write_text(scene_source + "\n" + interface.read_text())

        mixed = self.audit(self.scene)
        self.assertEqual(mixed.returncode, 1, mixed.stdout + mixed.stderr)
        self.assertIn("duration: 0.75", mixed.stdout)
        self.assertIn("duration: 0.2", mixed.stdout)

        self.scene.write_text(scene_source)
        separated = self.audit("--authored-source", self.scene, self.root)
        self.assertEqual(separated.returncode, 1, separated.stdout + separated.stderr)
        self.assertIn("TOKEN ROLE CHECKS EXCLUDED", separated.stdout)
        self.assertNotIn("duration: 0.75", separated.stdout)
        self.assertIn("duration: 0.2", separated.stdout)

        interface.write_text("animate(button, {duration: feedbackDuration, ease: productiveEase});")
        repaired = self.audit("--authored-source", self.scene, self.root)
        self.assertEqual(repaired.returncode, 0, repaired.stdout + repaired.stderr)

    def test_scene_selection_cannot_hide_an_unscanned_file_or_host(self):
        unscanned = self.root / "outside.js"
        unscanned.write_text(self.scene.read_text())
        result = self.audit("--authored-source", unscanned, self.scene)
        self.assertNotEqual(result.returncode, 0)
        host = self.root / "host.html"
        host.write_text('<a href="#">Missing destination</a>')
        result = self.audit("--authored-source", host, self.root)
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
