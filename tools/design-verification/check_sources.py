#!/usr/bin/env python3
"""Check links and caller-supplied disclosure terms in the given guidance files."""

import argparse
import re
import tomllib
import unicodedata
from pathlib import Path
from urllib.parse import unquote, urlsplit


MACHINE_PATH = re.compile(r"(/Users/[^/\s`]|/home/[a-z]|\b[A-Z]:\\|(?<![\w.$/~-])~/)")


def without_fences(text: str) -> str:
    """Remove fenced examples; their apparent links and headings are not rendered prose."""
    lines, marker, length = [], "", 0
    for line in text.splitlines():
        fence = re.match(r"^ {0,3}(`{3,}|~{3,})(.*)$", line)
        if not marker and fence:
            marker, length = fence.group(1)[0], len(fence.group(1))
        elif marker:
            if fence and fence.group(1)[0] == marker and len(fence.group(1)) >= length and not fence.group(2).strip():
                marker = ""
        else:
            lines.append(line)
    return "\n".join(lines)


def markdown_prose(text: str) -> str:
    """Exclude examples from link checks without discarding links in ordinary prose."""
    return re.sub(r"(`+)[^`\n]*?\1", "", without_fences(text))


def heading_anchors(text: str) -> dict[str, str]:
    """Map rendered heading ids to their visible titles, including duplicate suffixes."""
    ids = {}
    counts: dict[str, int] = {}
    for title in re.findall(r"^ {0,3}#{1,6}\s+(.+?)(?:\s+#+)?$", text, re.M):
        title = re.sub(r"\[([^]]+)\]\([^)]*\)", r"\1", title)
        title = re.sub(r"<[^>]*>", "", title).replace("`", "").lower()
        slug = "".join(c for c in title if c in "-_ " or unicodedata.category(c)[0] in "LN").replace(" ", "-")
        n = counts.get(slug, 0)
        counts[slug] = n + 1
        ids[f"{slug}-{n}" if n else slug] = title
    return ids


def markdown_anchors(text: str) -> set[str]:
    text = without_fences(text)
    explicit = re.findall(r'<(?:a|h[1-6])\b[^>]*\b(?:id|name)=[\'"]([^\'"]+)', text)
    return set(heading_anchors(text)).union(explicit)


MARKDOWN_LINK = re.compile(r"""\]\(\s*(?:<([^>]+)>|((?:\\.|[^\s()]|\([^()]*\))+))(?:\s+[\'"][^\n]*?[\'"])?\s*\)""")


def source_target(url: str) -> str:
    doi = re.match(r"https://doi\.org/(10\..+)$", url)
    return f"https://api.crossref.org/works/{doi.group(1)}" if doi else url


def source_results(urls: list[str]):
    """Schedule only eligible requests so one busy host cannot hold the worker pool."""
    from collections import Counter, deque
    from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
    from urllib.parse import urlsplit

    workers, per_host = 8, 2
    pending = {}
    for url in urls:
        host = urlsplit(source_target(url)).hostname
        pending.setdefault(host, deque()).append(url)
    ready = deque(pending)
    active = Counter()
    running = {}
    with ThreadPoolExecutor(max_workers=workers) as pool:
        while ready or running:
            while ready and len(running) < workers:
                host = ready.popleft()
                url = pending[host].popleft()
                running[pool.submit(check_source_url, url)] = host, url
                active[host] += 1
                if pending[host] and active[host] < per_host:
                    ready.append(host)
            completed, _ = wait(running, return_when=FIRST_COMPLETED)
            for future in completed:
                host, url = running.pop(future)
                if active[host] == per_host and pending[host]:
                    ready.append(host)
                active[host] -= 1
                yield url, future.result()


def check_source_fragment(response, fragment: str) -> tuple[str, str]:
    """Prove a named HTML destination or leave it for browser confirmation."""
    from html.parser import HTMLParser
    from urllib.parse import unquote, urlsplit

    target = unquote(urlsplit(response.geturl()).fragment or fragment)
    if (":~:" in target
            or response.headers.get_content_type() not in ("text/html", "application/xhtml+xml")):
        return "skipped", f"fragment #{target} requires browser confirmation"

    class FragmentTarget(HTMLParser):
        found = False
        template_depth = 0

        def handle_starttag(self, tag, attrs):
            if not self.template_depth and (("id", target) in attrs or (tag == "a" and ("name", target) in attrs)):
                self.found = True
            if tag == "template":
                self.template_depth += 1

        def handle_endtag(self, tag):
            if tag == "template" and self.template_depth:
                self.template_depth -= 1

    if target.lower() == "top":
        return "passed", "resolved to document top"
    limit = 8 * 1024 * 1024
    content = response.read(limit + 1)
    if len(content) > limit:
        return "skipped", f"fragment #{target} exceeds the HTML inspection limit"
    parser = FragmentTarget()
    encoding = response.headers.get_content_charset() or "utf-8"
    try:
        parser.feed(content.decode(encoding, errors="replace"))
    except LookupError:
        return "skipped", f"fragment #{target} uses an unknown document encoding"
    if parser.found:
        return "passed", "resolved to an HTML target"
    return "skipped", f"fragment #{target} absent from returned HTML; confirm in a browser"


def check_source_url(url: str) -> tuple[str, str]:
    """Separate a missing source from a challenge or a transient transport failure."""
    from email.utils import parsedate_to_datetime
    from math import ceil
    import time
    import urllib.error
    import urllib.request
    from urllib.parse import urlsplit
    req = urllib.request.Request(source_target(url), method="GET", headers={"User-Agent": "Mozilla/5.0 design-source-check"})
    fragment = urlsplit(url).fragment
    retriable = (429, 502, 503, 504)
    challenge = "skipped", "answered with a bot challenge; open it in a browser; not a pass"
    for attempt, pause in enumerate((2, 5, 0)):
        try:
            with urllib.request.urlopen(req, timeout=20) as response:
                if response.headers.get("cf-mitigated") == "challenge":
                    return challenge
                if response.status >= 400:
                    return "failed", f"returned {response.status}"
                if fragment:
                    return check_source_fragment(response, fragment)
            return "passed", "resolved"
        except urllib.error.HTTPError as error:
            headers = error.headers or {}
            challenged = headers.get("cf-mitigated") == "challenge"
            code = error.code
            retry_after = headers.get("Retry-After", "").strip()
            error.close()
            if challenged:
                return challenge
            if code in retriable and pause:
                delay = pause
                if retry_after:
                    try:
                        if re.fullmatch(r"[0-9]+", retry_after):
                            delay = int(retry_after)
                        else:
                            retry_at = parsedate_to_datetime(retry_after)
                            if retry_at.tzinfo is None:
                                raise ValueError("retry date lacks a time zone")
                            delay = max(0, ceil(retry_at.timestamp() - time.time()))
                    except (TypeError, ValueError, OverflowError, OSError):
                        return "skipped", f"returned {code}; invalid Retry-After, retry deferred"
                if delay > 10:
                    return "skipped", f"returned {code}; retry after {delay}s exceeds this check's wait budget"
                time.sleep(delay)
                continue
            if code in retriable:
                return "skipped", f"returned {code} after {attempt + 1} tries; reachability unresolved"
            return "failed", f"returned {code}"
        except (OSError, TimeoutError) as error:
            if not pause:
                return "skipped", f"transport unresolved after {attempt + 1} tries: {error}"
            time.sleep(pause)
    raise AssertionError("source check exhausted without an outcome")


def targets(files, root, *, machine_paths=False, private_terms=()):
    urls = {}
    findings = []
    private = re.compile(r"\b(?:" + "|".join(map(re.escape, private_terms)) + r")\b") if private_terms else None
    for filename in files:
        text = filename.read_text()
        display = filename.relative_to(root).as_posix()
        body = tomllib.loads(text).get("developer_instructions", "") if filename.suffix == ".toml" else text
        for number, line in enumerate(text.splitlines(), start=1):
            if machine_paths and MACHINE_PATH.search(line):
                findings.append(("failed", f"{display}:{number} contains a machine path"))
            if private and private.search(line):
                findings.append(("failed", f"{display}:{number} contains a private term"))
        if not isinstance(body, str):
            continue
        for match in MARKDOWN_LINK.finditer(markdown_prose(body)):
            href = match.group(1) or match.group(2)
            parts = urlsplit(href)
            if parts.scheme == "https" and parts.hostname:
                urls.setdefault(href, set()).add(display)
            elif not parts.scheme and not parts.netloc:
                relative = unquote(re.sub(r"\\([() ])", r"\1", parts.path))
                destination = (filename.parent / relative).resolve() if relative else filename
                if not destination.is_relative_to(root):
                    findings.append(("failed", f"{display} links outside the supplied root"))
                elif not destination.exists():
                    findings.append(("failed", f"{display} links to a missing path: {href}"))
                elif parts.fragment and destination.suffix.lower() == ".md" and unquote(parts.fragment) not in markdown_anchors(destination.read_text()):
                    findings.append(("failed", f"{display} links to a missing heading: {href}"))
    return urls, findings


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--online", action="store_true")
    parser.add_argument("--machine-paths", action="store_true")
    parser.add_argument("--private-terms", type=Path)
    parser.add_argument("files", type=Path, nargs="+")
    args = parser.parse_args()
    root = args.root.resolve()
    files = [p.resolve() for p in args.files]
    if not all(p.is_file() and p.is_relative_to(root) for p in files):
        parser.error("each file must exist within --root")
    terms = ()
    if args.private_terms:
        terms = tuple(line.strip() for line in args.private_terms.read_text().splitlines()
                      if line.strip() and not line.lstrip().startswith("#"))
        if not terms:
            parser.error("the private term list is empty")
    urls, findings = targets(files, root, machine_paths=args.machine_paths, private_terms=terms)
    if args.online:
        for url, (status, detail) in source_results(sorted(urls)):
            if status != "passed":
                findings.append((status, f"{url} {detail} ({', '.join(sorted(urls[url]))})"))
    elif urls:
        findings.append(("skipped", f"{len(urls)} external URLs. Use --online for these."))
    for status, message in sorted(findings):
        print(f"{status.upper()}: {message}")
    return 1 if any(status == "failed" for status, _ in findings) else 2 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
