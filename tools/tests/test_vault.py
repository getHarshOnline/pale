"""Exercise repairs on disposable trees; checks never alter the real vault."""

from contextlib import redirect_stderr, redirect_stdout
from datetime import date
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import vault


class VaultTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.documents = self.root / "knowledge"
        self.repositories = self.root / "repositories"
        self.documents.mkdir()
        self.repositories.mkdir()
        self.policy = vault.Policy((self.documents, self.repositories), (self.documents,),
                                   (self.documents, self.repositories))

    def write(self, path, text):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return path

    def command(self, repo, *args):
        return subprocess.run(["git", "-C", str(repo), *args], check=True,
                              capture_output=True).stdout

    def repository(self):
        repo = self.repositories / "product"
        repo.mkdir()
        self.command(repo, "init", "-q")
        self.command(repo, "config", "user.name", "Fixture")
        self.command(repo, "config", "user.email", "fixture@example.invalid")
        self.command(repo, "config", "commit.gpgsign", "false")
        return repo

    def commit(self, repo):
        self.command(repo, "add", ".")
        self.command(repo, "commit", "-qm", "Fixture")

    def codes(self, **kwargs):
        return {f.code for f in vault.check(self.policy, **kwargs)}

    def test_markdown_destinations_keep_exact_offsets(self):
        text = ('[one](<a b.md> "Title") [two](a(b).md#part)\n'
                '[three][ref]\n[ref]: ../c%20d.md "title"\n'
                '<https://example.invalid/test> <a href="e.md">E</a>\n'
                '`[ignored](absent.md)`\n```md\n[ignored](absent.md)\n```\n'
                '    [ignored](absent.md)\n<!-- [ignored](absent.md) -->\n')
        links = vault.markdown_links(text)
        self.assertEqual([link.target for link in links],
                         ["a b.md", "a(b).md#part", "../c%20d.md", "https://example.invalid/test", "e.md"])
        for link in links:
            self.assertEqual(text[link.start:link.end], link.target)

    def test_file_move_updates_both_roots_and_moved_page_links(self):
        old = self.write(self.documents / "old.md", '[asset](asset.txt)\n[self](#title)\n# Title\n')
        self.write(self.documents / "asset.txt", "original")
        incoming = self.write(self.repositories / "README.md", '[page](../knowledge/old.md#title)\n')
        new = self.documents / "deeper" / "new name.md"
        new.parent.mkdir()
        old.rename(new)
        plan = vault.plan_relink(self.policy, old, new)
        self.assertEqual(plan.findings, [])
        self.assertEqual(len(plan.changes), 2)
        self.assertIn("old.md", incoming.read_text())
        vault.apply_relink(self.policy, plan)
        self.assertIn("deeper/new%20name.md#title", incoming.read_text())
        self.assertIn("[asset](../asset.txt)", new.read_text())
        self.assertIn("[self](#title)", new.read_text())
        self.assertNotIn("missing-target", self.codes())

    def test_incoming_links_cross_roots_and_keep_destination_locations(self):
        target = self.write(self.documents / "a page.md", '# Page\n[self](#page)\n')
        source = self.write(self.repositories / "README.md",
                            '[inline](../knowledge/a%20page.md?view=one#page)\n'
                            '[reference][p]\n[p]: <../knowledge/a page.md>\n'
                            '<a href="../knowledge/a%20page.md#page">HTML</a>\n'
                            '[other](../knowledge/a%20page.md.bak)\n'
                            '`[code](../knowledge/a%20page.md)`\n'
                            '```md\n[code](../knowledge/a%20page.md)\n```\n')
        original = self.write(self.documents / 'Records' / 'original.md', '[page](../a%20page.md)\n')
        protected = self.write(self.repositories / 'client' / 'README.md', '[page](../../knowledge/a%20page.md)\n')
        self.policy.protected = (protected.parent,)
        self.policy.roots += (self.repositories,)
        (self.repositories / 'alias.md').symlink_to(source)
        before = {p: p.read_bytes() for p in (target, source, original, protected)}
        links, findings = vault.incoming_links(self.policy, target)
        self.assertEqual(findings, [])
        self.assertEqual([(link.line, link.kind) for link in links if link.path == str(source)],
                         [(1, 'markdown'), (3, 'markdown'), (4, 'html')])
        self.assertEqual({link.path for link in links if link.protected}, {str(original), str(protected)})
        self.assertEqual(len(links), 5)
        self.assertEqual(before, {p: p.read_bytes() for p in before})

    def test_incoming_links_find_missing_paths_and_cargo_references(self):
        missing = self.documents / 'missing'
        cargo = self.write(self.repositories / 'Cargo.toml', '[dependencies]\nthing = { path = "../knowledge/missing" }\n')
        links, findings = vault.incoming_links(self.policy, missing)
        self.assertEqual(findings, [])
        self.assertEqual([(link.path, link.line, link.kind) for link in links], [(str(cargo), 2, 'cargo')])
        with self.assertRaisesRegex(ValueError, 'declared search root'):
            vault.incoming_links(self.policy, self.root / 'outside.md')

    def test_incoming_cli_preserves_matches_when_a_file_cannot_be_read(self):
        target = self.documents / 'missing.md'
        source = self.write(self.documents / 'README.md', '[page](missing.md)\n')
        broken = self.documents / 'broken.md'
        broken.write_bytes(b'\xff')
        args = ['--root', str(self.documents), '--json', 'incoming', str(target)]
        output = io.StringIO()
        with redirect_stdout(output):
            status = vault.main(args)
        result = json.loads(output.getvalue())
        self.assertEqual(status, 1)
        self.assertEqual(result['target'], str(target))
        self.assertEqual(result['changes'], [])
        self.assertEqual([link['path'] for link in result['incoming']], [str(source)])
        self.assertEqual([finding['code'] for finding in result['findings']], ['read-error'])

    def test_incoming_cli_reports_empty_and_partial_scans_distinctly(self):
        target = self.documents / 'missing.md'
        args = ['--root', str(self.documents), 'incoming', str(target)]
        output = io.StringIO()
        with redirect_stdout(output):
            status = vault.main(args)
        self.assertEqual(status, 0)
        self.assertIn('0 incoming links in the selected roots', output.getvalue())
        failure = vault.Finding(str(self.documents), 1, 'read-error', 'Access denied')
        output = io.StringIO()
        with patch.object(vault, 'inventory', return_value=([], [], [failure])), redirect_stdout(output):
            status = vault.main(args)
        self.assertEqual(status, 1)
        self.assertIn('incomplete scan', output.getvalue())

    def test_directory_move_repairs_cargo_and_reference_definitions(self):
        old = self.documents / "bundle"
        self.write(old / "README.md", '[inside](inner.md)\n[back]: ../home.md\n')
        self.write(old / "inner.md", "# Inner\n")
        self.write(self.documents / "home.md", "Home")
        self.write(old / "Cargo.toml", '[dependencies]\ncore = { path = "../../repositories/core" }\n')
        cargo = self.write(self.repositories / "Cargo.toml", '[dependencies]\napp = { path = "../knowledge/bundle" }\n')
        (self.repositories / "core").mkdir()
        new = self.documents / "deeper" / "bundle"
        new.parent.mkdir()
        old.rename(new)
        plan = vault.plan_relink(self.policy, old, new)
        self.assertEqual(plan.findings, [])
        vault.apply_relink(self.policy, plan)
        self.assertIn("../knowledge/deeper/bundle", cargo.read_text())
        self.assertIn("../../../repositories/core", (new / "Cargo.toml").read_text())
        self.assertIn("[inside](inner.md)", (new / "README.md").read_text())
        self.assertIn("[back]: ../../home.md", (new / "README.md").read_text())

    def test_prefix_matches_path_components_only(self):
        new = self.write(self.documents / "new.md", "# New")
        incoming = self.write(self.documents / "index.md", '[keep](older.md) [change](old.md)')
        self.write(self.documents / "older.md", "Older")
        vault.apply_relink(self.policy, vault.plan_relink(self.policy, self.documents / "old.md", new))
        self.assertEqual(incoming.read_text(), '[keep](older.md) [change](new.md)')

    def test_html_queries_and_unicode_survive_relink(self):
        new = self.write(self.documents / "café.md", "# Café")
        source = self.write(self.documents / "README.md", '<a href="old.md?a=1&amp;b=2#caf%C3%A9">Link</a>')
        vault.apply_relink(self.policy, vault.plan_relink(self.policy, self.documents / "old.md", new))
        self.assertIn('caf%C3%A9.md?a=1&amp;b=2#caf%C3%A9', source.read_text())

    def test_original_and_client_links_block_apply(self):
        new = self.write(self.documents / "new.md", "New")
        source = self.write(self.documents / "Records" / "original.md", '[old](../old.md)')
        client = self.write(self.repositories / "client" / "README.md", '[old](../../knowledge/old.md)')
        self.policy.protected = (client.parent,)
        before = source.read_bytes(), client.read_bytes()
        plan = vault.plan_relink(self.policy, self.documents / "old.md", new)
        self.assertEqual([f.code for f in plan.findings], ["protected-repair", "protected-repair"])
        with self.assertRaises(ValueError):
            vault.apply_relink(self.policy, plan)
        self.assertEqual(before, (source.read_bytes(), client.read_bytes()))

    def test_stale_plan_changes_no_files(self):
        new = self.write(self.documents / "new.md", "New")
        first = self.write(self.documents / "a.md", '[old](old.md)')
        last = self.write(self.documents / "z.md", '[old](old.md)')
        plan = vault.plan_relink(self.policy, self.documents / "old.md", new)
        last.write_text("Changed by another writer")
        with self.assertRaisesRegex(ValueError, "File changed"):
            vault.apply_relink(self.policy, plan)
        self.assertEqual(first.read_text(), '[old](old.md)')

    def test_relink_requires_explicit_writable_roots_and_completed_move(self):
        old = self.write(self.documents / "old.md", "Old")
        new = self.write(self.documents / "new.md", "New")
        with self.assertRaisesRegex(ValueError, "after the move"):
            vault.plan_relink(self.policy, old, new)
        old.unlink()
        self.policy.writable = ()
        with self.assertRaisesRegex(ValueError, "writable"):
            vault.plan_relink(self.policy, old, new)

    def test_symlink_destinations_never_written(self):
        outside = self.write(self.root / "outside.md", "Outside")
        link = self.documents / "new.md"
        link.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "writable"):
            vault.plan_relink(self.policy, self.documents / "old.md", link)
        self.assertEqual(outside.read_text(), "Outside")

    def test_cargo_comments_and_examples_do_not_become_paths(self):
        text = ('[package]\nname = "fixture"\nversion = "0.0.0"\n'
                'description = "example path = \'bad\'"\n# path = "bad"\n'
                '[dependencies]\na = { path = "../actual" }\n')
        self.assertEqual([l.target for l in vault.cargo_links(text)], ["../actual"])

    def test_unclosed_inline_link_is_not_rewritten(self):
        self.assertEqual(vault.markdown_links('[text](old.md\n'), [])

    def test_footnote_body_is_not_a_link_destination(self):
        self.assertEqual(vault.markdown_links('[^1]: This is a footnote.\n'), [])

    def test_full_and_collapsed_references_must_be_defined(self):
        text = '[one][Absent]\n[two][]\n[three][KNOWN]\n[known]: there.md\n'
        self.assertEqual(vault.undefined_references(text), [1, 2])

    def test_inline_code_in_heading_contributes_to_anchor(self):
        self.assertIn('use-the-tool', vault.heading_ids('# Use the `tool`\n'))

    def test_sealed_material_keeps_its_links_as_written(self):
        sealed = self.write(self.documents / "study/production/notes.md", "[old](docs/gone.md)\n")
        source = self.write(self.documents / "study/source-library/brief.md", "[old](gone.md)\n")
        living = self.write(self.documents / "study/README.md", "[notes](production/notes.md)\n[gone](gone.md)\n")
        self.policy.protected = (sealed.parent,)
        self.policy.original_names = ("source-library",)
        missing = {f.path for f in vault.check(self.policy, suggest_moves=False) if f.code == "missing-target"}
        self.assertEqual(missing, {str(living)})
        self.assertNotIn(str(source), missing)

    def test_explicit_heading_attribute_supplies_its_own_anchor(self):
        ids = vault.heading_ids("## Follow a command through Weave's runtime {#follow-a-command}\n")
        self.assertIn('follow-a-command', ids)
        self.assertIn('follow-a-command-through-weaves-runtime', ids)

    def test_missing_link_in_empty_repository_still_reported(self):
        repo = self.repository()
        self.write(repo / "README.md", '[missing](old.md)')
        self.assertIn("missing-target", self.codes())

    def test_git_rename_chain_has_no_old_path_filter(self):
        repo = self.repository()
        old = self.write(repo / "old.md", "# Meaning\n" * 20)
        self.commit(repo)
        middle = repo / "middle.md"
        old.rename(middle)
        self.commit(repo)
        new = repo / "other" / "new.md"
        new.parent.mkdir()
        middle.rename(new)
        self.commit(repo)
        self.assertEqual(vault.Moves([repo]).candidates(old), (str(new),))

    def test_untracked_exact_content_and_ambiguous_copies(self):
        repo = self.repository()
        old = self.write(repo / "old.md", "# Original account\n")
        self.commit(repo)
        old.unlink()
        new = self.write(repo / "new.md", "# Original account\n")
        self.assertEqual(vault.Moves([repo]).candidates(old), (str(new),))
        duplicate = self.write(repo / "duplicate.md", new.read_text())
        self.assertEqual(vault.Moves([repo]).candidates(old), tuple(sorted((str(duplicate), str(new)))))
        duplicate.write_text("Different")
        new.write_text("Edited")
        self.assertEqual(vault.Moves([repo]).candidates(old), ())

    def test_last_committed_content_after_deletion(self):
        repo = self.repository()
        old = self.write(repo / "old.md", "# Older content\n")
        self.commit(repo)
        old.unlink()
        self.commit(repo)
        new = self.write(repo / "untracked.md", "# Older content\n")
        self.assertEqual(vault.Moves([repo]).candidates(old), (str(new),))

    def test_rename_candidates_are_reported_without_repair(self):
        repo = self.repository()
        old = self.write(repo / "old.md", "# Original\n")
        self.commit(repo)
        new = repo / "new.md"
        old.rename(new)
        incoming = self.write(self.documents / "README.md", '[missing](../repositories/product/old.md)')
        findings = vault.check(self.policy)
        missing = next(f for f in findings if f.code == "missing-target")
        self.assertEqual(missing.line, 1)
        self.assertEqual(missing.candidates, (str(new),))
        self.assertIn("old.md", incoming.read_text())

    def test_actual_broken_link_fails_without_change(self):
        path = self.write(self.documents / "README.md", '# Root\n\n[broken](absent.md)\n')
        before = path.read_bytes()
        self.assertIn("missing-target", self.codes())
        self.assertEqual(path.read_bytes(), before)

    def test_malformed_url_is_reported_instead_of_crashing_private_checks(self):
        self.write(self.documents / "README.md", '[bad](https://[broken)\n')
        self.assertIn("read-error", self.codes())

    def test_anchor_checks_and_private_heading_policy_are_distinct(self):
        self.write(self.documents / "README.md", '# Root\n[page](page.md#absent)\n')
        self.write(self.documents / "page.md", '# Present\n[back](README.md)\n')
        self.assertTrue({"private-heading-link", "missing-heading"} <= self.codes())
        self.write(self.repositories / "README.md", '# A\n# A\n[second](#a-1)\n')
        self.assertFalse(any(f.code == "missing-heading" and "repositories" in f.path for f in vault.check(self.policy)))

    def test_navigation_tags_aliases_and_google_home(self):
        self.write(self.documents / "README.md", '---\ntags: [project]\n---\n[page](page.md)\nhttps://docs.google.com/document/d/abc/edit\n')
        self.write(self.documents / "page.md", '---\ntags: [other, research/a, writing/b, engineering/c]\naliases: [Other]\n---\n[back](README.md)\nhttps://docs.google.com/document/d/abc/edit?x=1\n')
        self.write(self.documents / "Other.md", '---\ntags: [project]\n---\nNo route\n')
        codes = self.codes()
        self.assertTrue({"subject-tag", "topic-tags", "shared-alias", "source-home", "parent-route", "unreachable"} <= codes)

    def test_domain_hub_is_route_without_changing_subject(self):
        self.write(self.documents / "README.md", '---\ntags: [project]\n---\n[hub](Docs/Research/README.md)\n')
        self.write(self.documents / "Docs/Research/README.md", '---\ntags: [project]\n---\n[page](detail.md)\n[up](../../README.md)\n')
        self.write(self.documents / "Docs/Research/detail.md", '---\ntags: [project]\n---\n[up](README.md)\n')
        self.assertFalse({"unreachable", "parent-route", "subject-tag"} & self.codes())

    def test_explicit_footer_cannot_skip_its_readme(self):
        self.write(self.documents / "README.md", '[child](child/README.md)\n')
        self.write(self.documents / "child/README.md", '[page](page.md)\n[back](../README.md)\n')
        page = self.write(self.documents / "child/page.md", '[earlier link](README.md)\nPart of [root](../README.md).\n')
        self.assertTrue(any(f.code == "parent-route" and f.path == str(page) for f in vault.check(self.policy)))

    def test_metadata_quoted_commas_and_invalid_lists(self):
        parsed = vault.metadata('---\naliases: ["One, two", \'Three\']\ntags:\n  - subject\n  - research/topic\n---\n')
        self.assertEqual(parsed["aliases"], ["One, two", "Three"])
        with self.assertRaises(ValueError):
            vault.metadata('---\ntags: [broken\n---\n')

    def test_public_repo_cannot_link_private_tree(self):
        repo = self.repository()
        self.policy.public_roots = (repo,)
        self.write(self.documents / "private.md", "Private")
        self.write(repo / "README.md", '[private](../../knowledge/private.md)\nA /Users/example/private path.\n')
        self.assertTrue({"public-private-path", "public-local-link"} <= self.codes(suggest_moves=False))

    def test_private_repository_with_remote_keeps_private_links(self):
        repo = self.repository()
        self.command(repo, "remote", "add", "origin", "https://example.invalid/private.git")
        self.write(self.documents / "private.md", "Private")
        self.write(repo / "README.md", '[private](../../knowledge/private.md)\nA /Users/example/private path.\n')
        self.assertFalse({"public-private-path", "public-local-link"} & self.codes(suggest_moves=False))

    def test_cli_publication_scope_does_not_require_remote(self):
        repo = self.repository()
        self.write(repo / "README.md", 'A /Users/example/private path.\n')
        output = io.StringIO()
        with redirect_stdout(output):
            result = vault.main(["--root", str(repo), "--public-root", str(repo), "--json", "check"])
        self.assertEqual(result, 1)
        self.assertIn("public-private-path", {f["code"] for f in json.loads(output.getvalue())["findings"]})

    def test_list_continuation_links_are_checked_and_relinked(self):
        source = self.write(self.documents / "README.md", '- Parent\n    [Child](old.md)\n')
        self.assertTrue(any(f.code == "missing-target" and f.line == 2 for f in vault.check(self.policy)))
        new = self.write(self.documents / "new.md", "# New")
        vault.apply_relink(self.policy, vault.plan_relink(self.policy, self.documents / "old.md", new))
        self.assertEqual(source.read_text(), '- Parent\n    [Child](new.md)\n')

    def test_list_code_blocks_stay_out_of_link_repairs(self):
        text = ('- Parent\n    [Child](child.md)\n\n'
                '      [code](ignore.md)\n\n'
                '    - Nested\n      [Grandchild](grandchild.md)\n\n'
                '          [code](ignore.md)\n\n'
                '- Next\n\n  ```md\n  [code](ignore.md)\n  ```\n\n'
                'Paragraph outside the list.\n\n    [code](ignore.md)\n')
        self.assertEqual([link.target for link in vault.markdown_links(text)], ['child.md', 'grandchild.md'])

    def test_inline_code_cannot_supply_an_html_anchor(self):
        self.write(self.repositories / "README.md", '`<a id="missing"></a>`\n[go](#missing)\n')
        self.assertIn("missing-heading", self.codes())
        text = '# Use `tool`\n<a id="real"></a>\n`<a id="fake"></a>`\n'
        self.assertEqual(vault.heading_ids(text), {"use-tool", "real"})
        self.assertNotIn("escaped", vault.heading_ids('`&lt;a id="escaped"&gt;&lt;/a&gt;`\n'))

    def test_parked_output_checks_use_task_end_not_file_age(self):
        working = self.documents / "Codex"
        task = self.write(working / "finished" / "note.md", "Working note")
        self.write(self.documents / "README.md", '[working](Codex/finished/note.md)\n[chat](codex://threads/example)\nReasoning is in [records](Records/README.md).\n')
        self.write(self.documents / "Records/README.md", 'inode: 456\n')
        (self.documents / "Records/Evidence").mkdir()
        build = self.write(self.repositories / "build/output.bin", "12345")
        hook = self.write(self.root / "hooks.json", '{"command": "old-surface check"}')
        self.policy.working_areas = (working,)
        self.policy.task_ends = {task.parent: date(2020, 1, 1)}
        self.policy.build_roots = (build.parent,)
        self.policy.build_limit = 4
        self.policy.hooks = (hook,)
        self.policy.retired_commands = ("old-surface",)
        native_size = vault.BuildSize(str(build.parent), 5, 1, 0, True, ())
        with patch.object(vault, "measure", return_value=native_size):
            self.assertTrue({"working-link", "reason-in-records", "record-metadata", "parked-evidence", "ended-task-output", "build-size", "retired-hook"} <= self.codes())
        self.policy.task_ends = {}
        self.assertNotIn("ended-task-output", self.codes())

    def test_cli_is_read_only_and_roots_are_independent_of_cwd(self):
        path = self.write(self.documents / "README.md", '[broken](absent.md)')
        output = io.StringIO()
        previous = Path.cwd()
        try:
            os.chdir(self.repositories)
            with redirect_stdout(output):
                result = vault.main(["--root", str(self.documents), "--json", "check", "--no-move-suggestions"])
        finally:
            os.chdir(previous)
        self.assertEqual(result, 1)
        self.assertEqual(json.loads(output.getvalue())["findings"][0]["path"], str(path))
        self.assertEqual(path.read_text(), '[broken](absent.md)')

    def test_cli_invalid_root_is_error(self):
        with redirect_stderr(io.StringIO()):
            result = vault.main(["--root", str(self.root / "absent"), "check"])
        self.assertEqual(result, 2)

    def tag_policy(self):
        standard = self.write(self.documents / "standard.md",
                              '| Root | Topics |\n| --- | --- |\n'
                              '| `engineering` | code |\n| `design` | appearance |\n'
                              '| `family` | care |\n')
        self.write(self.documents / "owner" / "README.md", '---\ntags: [owner]\n---\n# Owner\n')
        graph = self.write(self.documents / ".obsidian" / "graph.json", json.dumps({
            "scale": 0.03902778042419557, "showTags": False,
            "colorGroups": [
                {"query": "tag:#engineering -tag:#design", "color": {"a": 1, "rgb": 31487}},
                {"query": 'tag:#engineering path:"Personal"', "color": {"a": .5, "rgb": 3}},
                {"query": "tag:#personal", "color": {"a": 1, "rgb": 8}},
                {"query": "tag:#design -tag:#private", "color": {"a": 1, "rgb": 9}},
            ]}, indent=2))
        policy = vault.Policy((self.documents,), vaults=(self.documents,), writable=(self.documents,),
                              topic_standard=standard, graphs=(graph,))
        return policy, standard, graph

    def test_topic_table_rejects_missing_duplicate_or_malformed_roots(self):
        for text in ('No table', '| Root | Topics |\n| --- | --- |\n',
                     '| Root | Topics |\n| --- | --- |\n| `code` | one |\n| `code` | two |\n',
                     '| Root | Topics |\n| --- | --- |\n| `Bad Root` | one |\n'):
            with self.subTest(text=text), self.assertRaises(ValueError):
                vault.topic_table(text)

    def test_graph_uses_table_and_preserves_preferences_and_custom_groups(self):
        policy, standard, graph = self.tag_policy()
        before = json.loads(graph.read_text())
        plan = vault.plan_tags(policy)
        self.assertEqual(graph.read_text(), plan.changes[0].before.decode())
        vault.apply_relink(policy, plan)
        after = json.loads(graph.read_text())
        expected = json.loads(json.dumps(before))
        expected['colorGroups'][0]['query'] = 'tag:#engineering -tag:#design -tag:#family'
        expected['colorGroups'][3]['query'] = 'tag:#design -tag:#engineering -tag:#family -tag:#private'
        self.assertEqual(after, expected)
        self.assertEqual(vault.plan_tags(policy).changes, [])
        # A new root changes exclusions without editing code or graph configuration.
        standard.write_text(standard.read_text() + '| `research` | inquiry |\n')
        plan = vault.plan_tags(policy)
        vault.apply_relink(policy, plan)
        self.assertEqual(json.loads(graph.read_text())['colorGroups'][0]['query'],
                         'tag:#engineering -tag:#design -tag:#family -tag:#research')

    def test_subject_difference_is_review_only_and_can_be_explicitly_retained(self):
        policy, _, _ = self.tag_policy()
        page = self.write(self.documents / 'owner' / 'Docs' / 'Brand.md',
                          '---\ntags: [owner/brand, design]\n---\n# Brand\n')
        plan = vault.plan_tags(policy)
        reviews = [f for f in plan.findings if f.code == 'subject-tag']
        self.assertTrue(reviews and all(f.review for f in reviews))
        self.assertNotIn(page, [c.path for c in plan.changes])
        retained = vault.plan_tags(policy, {page: 'owner/brand'})
        self.assertFalse(any(f.path == str(page) for f in retained.findings))
        self.assertNotIn(page, [c.path for c in retained.changes])
        vault.apply_relink(policy, plan)
        self.assertEqual(vault.metadata(page.read_text())['tags'][0], 'owner/brand')

    def test_explicit_subject_repair_preserves_manual_tags_comments_and_body(self):
        policy, _, _ = self.tag_policy()
        before = ('---\ndescription: Keep this.\ntags:\n'
                  '  # The subject follows.\n  - wrong # retained comment\n'
                  '  - engineering/code\n  - personal-label\naliases: [One]\n---\nBody stays.\n')
        page = self.write(self.documents / 'owner' / 'Docs' / 'Page.md', before)
        plan = vault.plan_tags(policy, {page: 'owner'})
        vault.apply_relink(policy, plan)
        self.assertEqual(page.read_text(), before.replace('- wrong #', '- owner #'))

    def test_missing_subject_preserves_existing_topic_and_inline_manual_tags(self):
        for source in ('---\ntags: [engineering/code, manual-label]\n---\nBody\n',
                       '---\ntags:\n  - engineering/code\n  - manual-label\n---\nBody\n',
                       '---\ntype: guide\n---\nBody\n'):
            after = vault.set_subject(source, 'owner', ('engineering',))
            self.assertEqual(vault.metadata(after)['tags'],
                             ['owner', 'engineering/code', 'manual-label'] if 'tags:' in source else ['owner'])
            self.assertTrue(after.endswith('---\nBody\n'))

    def test_tag_repairs_reject_originals_clients_and_symlinks(self):
        policy, _, graph = self.tag_policy()
        original = self.write(self.documents / 'owner' / 'Docs' / 'Records' / 'Original.md', '---\ntags: [old]\n---\n')
        client = self.write(self.documents / 'client' / 'Docs' / 'Page.md', original.read_text())
        link = self.documents / 'linked.md'
        link.symlink_to(client)
        policy.protected = (client.parent.parent,)
        before = graph.read_bytes(), original.read_bytes(), client.read_bytes()
        plan = vault.plan_tags(policy, {original: 'owner', client: 'owner', link: 'owner'})
        self.assertEqual(sum(f.code == 'protected-repair' for f in plan.findings), 3)
        with self.assertRaises(ValueError):
            vault.apply_relink(policy, plan)
        self.assertEqual(before, (graph.read_bytes(), original.read_bytes(), client.read_bytes()))

    def test_tag_apply_rejects_stale_standard_or_graph_before_writing(self):
        policy, standard, graph = self.tag_policy()
        page = self.write(self.documents / 'owner' / 'Docs' / 'Page.md', '---\ntags: [old]\n---\n')
        original = page.read_bytes()
        plan = vault.plan_tags(policy, {page: 'owner'})
        standard.write_text(standard.read_text() + '| `research` | inquiry |\n')
        with self.assertRaisesRegex(ValueError, 'Repair input changed'):
            vault.apply_relink(policy, plan)
        self.assertEqual(page.read_bytes(), original)
        plan = vault.plan_tags(policy, {page: 'owner'})
        graph.write_text('{}')
        with self.assertRaisesRegex(ValueError, 'File changed'):
            vault.apply_relink(policy, plan)
        self.assertEqual(page.read_bytes(), original)

    def test_tag_cli_review_is_success_and_apply_is_explicit(self):
        policy, standard, graph = self.tag_policy()
        page = self.write(self.documents / 'owner' / 'Docs' / 'Brand.md', '---\ntags: [owner/brand]\n---\n')
        before = graph.read_bytes(), page.read_bytes()
        args = ['--root', str(self.documents), '--vault', str(self.documents), '--writable', str(self.documents),
                '--topic-standard', str(standard), '--graph', str(graph), '--json', 'tags']
        output = io.StringIO()
        with redirect_stdout(output):
            result = vault.main(args)
        self.assertEqual(result, 0)
        self.assertTrue(all(f['review'] for f in json.loads(output.getvalue())['findings']))
        self.assertEqual(before, (graph.read_bytes(), page.read_bytes()))
        with redirect_stdout(io.StringIO()):
            result = vault.main([*args, '--subject', str(page) + '=owner/brand', '--apply'])
        self.assertEqual(result, 0)
        self.assertEqual(page.read_bytes(), before[1])
        self.assertNotEqual(graph.read_bytes(), before[0])


if __name__ == "__main__":
    unittest.main()
