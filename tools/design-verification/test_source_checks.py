"""Link verification preserves distinct failed, skipped and retriable outcomes."""
import importlib.util
from email.message import Message
from email.utils import formatdate
from pathlib import Path
from tempfile import TemporaryDirectory
from threading import Event, Lock
import unittest
from unittest.mock import patch, MagicMock
from urllib.error import HTTPError

spec = importlib.util.spec_from_file_location('harness', Path(__file__).with_name('check_sources.py'))
harness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(harness)


class SourceChecks(unittest.TestCase):
    def test_online_sweep_checks_instruction_links_and_source_lists_once(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            skills = root / ".agents/skills"
            reference = skills / "example/references/topic.md"
            reference.parent.mkdir(parents=True)
            reference.write_text(
                "# Topic\n\nRead [Guide](https://example.test/guide).\n"
                "`[Inline example](https://example.test/inline-code)`\n\n"
                "```md\n[Example](https://example.test/fenced)\n```\n\n"
                "## Sources\n\n- [Guide](https://example.test/guide): fetch for the topic.\n")
            (root / "AGENTS.md").write_text("[Host guidance](https://example.test/host)\n")
            (root / ".codex/agents").mkdir(parents=True)
            (root / ".codex/agents/critic.toml").write_text(
                'developer_instructions = "[Role guide](https://example.test/role) '
                '[Shared guide](https://example.test/guide)"\n')
            role = root / ".codex/agents/critic.toml"
            urls, findings = harness.targets([reference, root / "AGENTS.md", role], root)
            self.assertEqual(findings, [])
            self.assertEqual(set(urls), {
                "https://example.test/guide", "https://example.test/host", "https://example.test/role",
            })
            self.assertEqual(urls["https://example.test/guide"], {
                ".codex/agents/critic.toml", ".agents/skills/example/references/topic.md",
            })

    def test_missing_anchor_and_private_term_are_reported(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            doc = root / "read.md"
            doc.write_text("# Heading\n\n[Broken](#missing)\nPrivateMarker\n")
            _, findings = harness.targets([doc], root, private_terms=["PrivateMarker"])
            self.assertEqual(len(findings), 2)
            self.assertTrue(any("missing heading" in message for _, message in findings))
            self.assertTrue(any("contains a private term" in message for _, message in findings))
            self.assertNotIn("PrivateMarker", str(findings))

    def test_links_accept_spaced_paths_and_require_real_headings(self):
        with TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            target = root / "Reference page.md"
            target.write_text("# Entry\n\n```md\n# Entry\n```\n")
            source = root / "guide.md"
            source.write_text(
                "[Spaced path](<Reference page.md#entry>)\n"
                "[Encoded path](Reference%20page.md#entry)\n"
                "[Second entry](<Reference page.md#entry-1>)\n"
                "[Missing file](missing.md#entry)\n"
            )
            _, findings = harness.targets([source], root)
            self.assertEqual(findings, [
                ("failed", "guide.md links to a missing heading: Reference page.md#entry-1"),
                ("failed", "guide.md links to a missing path: missing.md#entry"),
            ])

    def test_host_instructions_flag_machine_paths_without_echoing_them(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            role = root / "reviewer.toml"
            role.write_text('developer_instructions = "Open ~/private-copy or /Users/example/engine."\n')
            _, findings = harness.targets([role], root, machine_paths=True)
            self.assertEqual(findings, [("failed", "reviewer.toml:1 contains a machine path")])

    def test_one_slow_host_cannot_starve_a_later_host(self):
        other_host_started = Event()
        lock = Lock()
        active = 0
        peak = 0
        urls = [f"https://a.test/{n}" for n in range(20)] + ["https://z.test/guide"]

        def resolve(url):
            nonlocal active, peak
            if url.startswith("https://z.test/"):
                other_host_started.set()
                return "passed", "resolved"
            with lock:
                active += 1
                peak = max(peak, active)
            try:
                if not other_host_started.wait(timeout=2):
                    raise AssertionError("a later host never received a worker")
                return "passed", "resolved"
            finally:
                with lock:
                    active -= 1

        with patch.object(harness, "check_source_url", side_effect=resolve):
            self.assertCountEqual([url for url, _ in harness.source_results(urls)], urls)
        self.assertLessEqual(peak, 2)

    def test_success_and_doi_target(self):
        response = MagicMock()
        response.__enter__.return_value.status = 200
        with patch('urllib.request.urlopen', return_value=response) as request:
            self.assertEqual(harness.check_source_url('https://doi.org/10.1/example'), ('passed', 'resolved'))
        self.assertEqual(request.call_args.args[0].full_url, 'https://api.crossref.org/works/10.1/example')

    def test_html_success_does_not_clear_a_missing_fragment(self):
        url = "https://example.test/guide#exact"
        response = MagicMock()
        returned = response.__enter__.return_value
        returned.status = 200
        returned.geturl.return_value = "https://example.test/guide"
        returned.headers = Message()
        returned.headers["Content-Type"] = "text/html; charset=utf-8"
        for content, status in (
            (b'<h2 id="exact">Topic</h2>', "passed"),
            (b'<a name="exact"></a>', "passed"),
            (b'<h2 id="other">Topic</h2>', "skipped"),
            (b'<template><h2 id="exact">Inert</h2></template>', "skipped"),
            (b'<!-- <h2 id="exact">Comment</h2> -->', "skipped"),
        ):
            with self.subTest(content=content), patch("urllib.request.urlopen", return_value=response):
                returned.read.return_value = content
                self.assertEqual(harness.check_source_url(url)[0], status)

    def test_redirect_fragment_and_non_html_scope(self):
        response = MagicMock()
        returned = response.__enter__.return_value
        returned.status = 200
        returned.geturl.return_value = "https://example.test/new#new-heading"
        returned.headers = Message()
        returned.headers["Content-Type"] = "text/html"
        returned.read.return_value = b'<h2 id="new-heading">Topic</h2>'
        with patch("urllib.request.urlopen", return_value=response):
            self.assertEqual(harness.check_source_url("https://example.test/old#old-heading")[0], "passed")
            returned.headers.replace_header("Content-Type", "application/pdf")
            self.assertEqual(harness.check_source_url("https://example.test/guide.pdf#page=2")[0], "skipped")

    def test_missing_page_is_failed_without_retry(self):
        error = HTTPError('https://example.test', 404, 'missing', {}, None)
        with patch('urllib.request.urlopen', side_effect=error) as request:
            self.assertEqual(harness.check_source_url('https://example.test')[0], 'failed')
        self.assertEqual(request.call_count, 1)

    def test_challenge_is_not_reported_as_missing_or_passed(self):
        error = HTTPError('https://example.test', 403, 'challenge', {'cf-mitigated': 'challenge'}, None)
        with patch('urllib.request.urlopen', side_effect=error):
            self.assertEqual(harness.check_source_url('https://example.test')[0], 'skipped')

    def test_successful_status_with_challenge_header_leaves_source_unverified(self):
        response = MagicMock()
        returned = response.__enter__.return_value
        returned.status = 200
        returned.headers = Message()
        returned.headers["Cf-Mitigated"] = "challenge"
        returned.headers["Content-Type"] = "text/html"
        returned.geturl.return_value = "https://example.test/guide"
        returned.read.return_value = b'<h2 id="exact">Interstitial content</h2>'
        with patch("urllib.request.urlopen", return_value=response) as request:
            for url in ("https://example.test/guide", "https://example.test/guide#exact"):
                with self.subTest(url=url):
                    status, detail = harness.check_source_url(url)
                    self.assertEqual(status, "skipped")
                    self.assertIn("bot challenge", detail)
        self.assertEqual(request.call_count, 2)
        returned.read.assert_not_called()

    def test_transport_recovery_and_exhaustion(self):
        response = MagicMock()
        response.__enter__.return_value.status = 200
        with patch('urllib.request.urlopen', side_effect=[TimeoutError('slow'), response]) as request, patch('time.sleep'):
            self.assertEqual(harness.check_source_url('https://example.test')[0], 'passed')
            self.assertEqual(request.call_count, 2)
        with patch('urllib.request.urlopen', side_effect=TimeoutError('slow')) as request, patch('time.sleep'):
            status, detail = harness.check_source_url('https://example.test')
            self.assertEqual(status, 'skipped')
            self.assertIn('3 tries', detail)
            self.assertEqual(request.call_count, 3)

    def test_rate_limit_retries_and_honours_short_retry_after(self):
        error = HTTPError('https://example.test', 429, 'limited', {'Retry-After': '3'}, None)
        response = MagicMock()
        response.__enter__.return_value.status = 200
        with patch('urllib.request.urlopen', side_effect=[error, response]) as request, patch('time.sleep') as sleep:
            self.assertEqual(harness.check_source_url('https://example.test'), ('passed', 'resolved'))
        self.assertEqual(request.call_count, 2)
        sleep.assert_called_once_with(3)

    def test_rate_limit_exhaustion_leaves_source_unresolved(self):
        with patch('urllib.request.urlopen', side_effect=HTTPError('https://example.test', 429, 'limited', {}, None)) as request, patch('time.sleep'):
            self.assertEqual(harness.check_source_url('https://example.test')[0], 'skipped')
        self.assertEqual(request.call_count, 3)

    def test_gateway_timeout_is_distinct_from_missing(self):
        error = HTTPError('https://example.test', 504, 'gateway timeout', {}, None)
        with patch('urllib.request.urlopen', side_effect=error) as request, patch('time.sleep'):
            status, detail = harness.check_source_url('https://example.test')
        self.assertEqual(status, 'skipped')
        self.assertIn('504 after 3 tries', detail)
        self.assertEqual(request.call_count, 3)

    def test_long_retry_after_is_reported_without_early_retry(self):
        error = HTTPError('https://example.test', 429, 'limited', {'Retry-After': '120'}, None)
        with patch('urllib.request.urlopen', side_effect=error) as request, patch('time.sleep') as sleep:
            status, detail = harness.check_source_url('https://example.test')
        self.assertEqual(status, 'skipped')
        self.assertIn('120s', detail)
        self.assertEqual(request.call_count, 1)
        sleep.assert_not_called()

    def test_http_date_respects_bounded_retry_budget(self):
        now = 1_800_000_000
        response = MagicMock()
        response.__enter__.return_value.status = 200
        for delay in (4, 120):
            with self.subTest(delay=delay):
                error = HTTPError('https://example.test', 503, 'busy',
                                  {'Retry-After': formatdate(now + delay, usegmt=True)}, None)
                with (patch('urllib.request.urlopen', side_effect=[error, response]) as request,
                      patch('time.time', return_value=now), patch('time.sleep') as sleep):
                    self.assertEqual(harness.check_source_url('https://example.test')[0],
                                     'passed' if delay == 4 else 'skipped')
                if delay == 4:
                    sleep.assert_called_once_with(4)
                    self.assertEqual(request.call_count, 2)
                else:
                    sleep.assert_not_called()
                    self.assertEqual(request.call_count, 1)

    def test_invalid_non_ascii_retry_after_never_raises_or_retries(self):
        error = HTTPError('https://example.test', 429, 'limited', {'Retry-After': '²'}, None)
        with patch('urllib.request.urlopen', side_effect=error) as request, patch('time.sleep') as sleep:
            self.assertEqual(harness.check_source_url('https://example.test')[0], 'skipped')
        self.assertEqual(request.call_count, 1)
        sleep.assert_not_called()


if __name__ == '__main__':
    unittest.main()
