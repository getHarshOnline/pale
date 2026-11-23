"""Check Package navigation and port comparisons."""

from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from package_reader import package_command, read_packages
from package_text import compare, port_report, register, regressions, word_pattern


class PackageStructureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = read_packages(ROOT)
        cls.resources = {f"{r['package']}:{r['id']}": r for r in cls.catalog["resources"]}

    def test_reasoning_opening_follows_edges_not_declaration_order(self):
        import shutil
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ["engineering", "governance"]:
                shutil.copytree(ROOT / "packages" / name, root / "packages" / name)
            path = root / "packages/engineering/governing-order.toml"
            prefix, *edges = path.read_text().split("[[governing_order.order]]")
            path.write_text(prefix + "".join("[[governing_order.order]]" + edge for edge in reversed(edges)))
            catalog = read_packages(root)
            opening = next(i["content"] for i in catalog["indexes"] if i["path"] == "engineering/INDEX.md")
            self.assertIn("](<change.md>)", opening)
            expected = ["change/impact", "change/constraints", "change/choice",
                        "testing", "review", "change/limits"]
            positions = [opening.index(f"](<{name}.md>)") for name in expected]
            self.assertEqual(positions, sorted(positions))
            path.write_text(path.read_text() + """
[[governing_order.order]]
purpose = "reasoning"
scope = { scope = "local", resource_id = "change", kind = "role" }
before = { scope = "local", resource_id = "change/limits", kind = "policy_family" }
after = { scope = "local", resource_id = "change/impact", kind = "policy_family" }
""")
            with self.assertRaises(ValueError):
                read_packages(root)

    def test_every_index_matches_the_compiler_view(self):
        self.assertEqual(len(self.catalog["indexes"]), 14)
        for index in self.catalog["indexes"]:
            with self.subTest(path=index["path"]):
                self.assertEqual(index["current"], index["content"])

    def test_body_preview_puts_typed_reading_links_before_authored_prose(self):
        resource = self.resources["pale.design:inspect"]
        text = package_command(ROOT, "show", "pale.design:inspect")
        _, opening = resource["text"].lstrip().split("\n", maxsplit=1)
        before, body = text.split(opening.lstrip(), maxsplit=1)
        self.assertEqual(body, "")
        self.assertIn(f"Read first: [{self.resources['pale.design:access/web']['title']}](<access/web.md>)", before)
        self.assertIn("Read as needed:", before)
        self.assertIn("](<animate.md>)", before)

    def test_index_shows_declared_reading_even_when_a_page_has_a_task_table(self):
        index = next(i["content"] for i in self.catalog["indexes"] if i["path"] == "design/INDEX.md")
        self.assertIn("](<animate.md>)", index)
        self.assertIn("](<animate/time-models.md>)", index)


class PackageTextTests(unittest.TestCase):
    def test_register_skips_code_and_reports_comparison_warnings(self):
        vocabulary = word_pattern("widget*\ngadget\n")
        old = {"entry.md": register("Read the current page.", vocabulary)}
        prose = "Compare widgets with gadget.\n\nUse `gadget`.\n\n```\nwidget\n```\n"
        new = {"entry.md": register(prose, vocabulary)}
        self.assertEqual(new["entry.md"]["flagged"], 2)
        self.assertEqual(len(new["entry.md"]["dense_sentences"]), 1)
        self.assertEqual(list(regressions(new, old)), ["entry.md"])

    def test_port_compares_each_kind_and_requires_a_reason_for_large_drops(self):
        source = "You must run `anim.play` for 16ms. [API](api.md)\n\n```js\nplay()\n```\n"
        result = compare(source, "Open it.")
        self.assertTrue(result["drop_over_20_percent"])
        self.assertEqual(set(result["missing"]), {
            "identifiers", "numbers", "link_targets", "code_blocks", "rules",
        })
        self.assertTrue(all(result["missing"].values()))
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "old.md").write_text(source)
            (root / "new.md").write_text("Open it.")
            mapping = [{"source": str(root / "old.md"), "destinations": ["new.md"]}]
            self.assertIn("word_count", port_report(mapping, root)[0]["unexplained"])
            mapping[0]["reasons"] = dict.fromkeys(
                [*result["missing"], "word_count"], "Moved to the linked manual.")
            self.assertEqual(port_report(mapping, root)[0]["unexplained"], [])


if __name__ == "__main__":
    unittest.main()
