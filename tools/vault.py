#!/usr/bin/env python3
"""Check local knowledge, repair links after a move, or reconcile selected tags.

See docs/vault.md for commands, workspace inputs and repair limits.
Callers supply workspace policy through Policy; this module owns no machine
configuration. `check` never writes. `relink OLD NEW` previews changes after a
file or directory has moved. Each invocation recomputes the plan; --apply writes
the reviewed current plan without moving or deleting material. Git history
suggests candidates, never an automatic repair.
`tags` derives graph queries from a supplied topic table. Subject differences
need review; --subject PAGE=TAG selects a correction or retains its own subject.
`builds` reports every --build-root without scanning knowledge pages. For example:
python3 tools/vault.py --root . --pale-executable /absolute/path/to/pale --build-root target --json builds
Totals are logical regular-file bytes, not reclaimable disk space. Symlinks are
excluded and incomplete scans are errors. --build-limit uses the same byte unit.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from dataclasses import asdict, dataclass, field
from datetime import date
import html
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
import tomllib
import unicodedata
from urllib.parse import quote, unquote, urlsplit

from build_sizes import BuildSize, measure


SKIP = frozenset({".git", "node_modules", "target", "build", "dist", ".venv",
                  "venv", "__pycache__", ".codegraph", ".pale-stage"})
ORIGINALS = frozenset({"Records", "Sources"})
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
PRIVATE_PATH = re.compile(r"(?:file://|/Users/|/home/|~[/\\]|[A-Za-z]:\\Users\\)")
GOOGLE = re.compile(r"https://(?:docs|drive)\.google\.com/[^\s<>\])]+")
MAX_TEXT = 2 * 1024 * 1024


def absolute(path: Path | str) -> Path:
    """Normalize without following symlinks, including paths that have moved."""
    return Path(os.path.abspath(os.path.expanduser(str(path))))


def within(path: Path, root: Path) -> bool:
    return path == root or root in path.parents


@dataclass
class Policy:
    """Workspace inputs, shared by a CLI caller and a future doctor integration.

    roots are the complete search trees, independent of the working directory.
    vaults receive private-page conventions. Only writable roots may be repaired;
    protected roots and original/source folders always remain read-only. Node
    roots and subject tags may be supplied where README placement is ambiguous.
    public_roots explicitly selects the trees intended for publication; a Git
    remote says nothing about that choice. Task end dates are explicit: a folder's
    age does not establish task completion.
    """

    roots: tuple[Path, ...]
    vaults: tuple[Path, ...] = ()
    writable: tuple[Path, ...] = ()
    protected: tuple[Path, ...] = ()
    nodes: dict[Path, str] = field(default_factory=dict)
    topic_roots: tuple[str, ...] = ()
    working_areas: tuple[Path, ...] = ()
    task_ends: dict[Path, date] = field(default_factory=dict)
    source_homes: dict[str, Path] = field(default_factory=dict)
    build_roots: tuple[Path, ...] = ()
    build_limit: int | None = None
    pale_executable: Path | None = None
    hooks: tuple[Path, ...] = ()
    original_names: tuple[str, ...] = ()
    retired_commands: tuple[str, ...] = ()
    today: date = field(default_factory=date.today)
    public_roots: tuple[Path, ...] = ()
    topic_standard: Path | None = None
    graphs: tuple[Path, ...] = ()

    def __post_init__(self):
        for name in ("roots", "vaults", "writable", "protected", "working_areas",
                     "build_roots", "hooks", "public_roots", "graphs"):
            setattr(self, name, tuple(absolute(p) for p in getattr(self, name)))
        self.nodes = {absolute(p): tag for p, tag in self.nodes.items()}
        self.task_ends = {absolute(p): end for p, end in self.task_ends.items()}
        self.source_homes = {key: absolute(p) for key, p in self.source_homes.items()}
        if not self.roots or any(not p.is_dir() for p in self.roots):
            raise ValueError("Supply existing search directories with --root.")
        if any(not any(within(p, r) for r in self.roots)
               for p in (*self.vaults, *self.writable, *self.nodes, *self.public_roots)):
            raise ValueError("Vault, writable, node and publication roots must be inside search roots.")
        if self.build_limit is not None and self.build_limit < 0:
            raise ValueError("The build size limit must be nonnegative.")
        if self.topic_standard is not None:
            self.topic_standard = absolute(self.topic_standard)
            roots = topic_table(read_text(self.topic_standard))
            if self.topic_roots and self.topic_roots != roots:
                raise ValueError("Topic roots disagree with the supplied standard.")
            self.topic_roots = roots
        if self.graphs and self.topic_standard is None:
            raise ValueError("Graph queries require a topic standard.")

    def private(self, path: Path) -> bool:
        return any(within(path, root) for root in self.vaults)

    def publication_root(self, path: Path) -> Path | None:
        return max((root for root in self.public_roots if within(path, root)),
                   key=lambda root: len(root.parts), default=None)

    def original(self, path: Path) -> bool:
        names = ORIGINALS.union(self.original_names)
        return any(within(path, root) and names.intersection(path.relative_to(root).parts)
                   for root in self.roots)

    def sealed(self, path: Path) -> bool:
        """Original or protected material keeps its links as written, so the check reads past them."""
        return self.original(path) or any(within(path, root) for root in self.protected)

    def living(self, path: Path) -> bool:
        return self.private(path) and not self.original(path) and not any(
            within(path, root) for root in self.working_areas)

    def can_write(self, path: Path) -> bool:
        return (any(within(path, root) for root in self.writable)
                and not self.original(path)
                and not any(within(path, root) for root in self.protected)
                and not any(part.is_symlink() for part in (path, *path.parents)))


@dataclass(frozen=True, order=True)
class Finding:
    path: str
    line: int
    code: str
    message: str
    candidates: tuple[str, ...] = ()
    target: str = ""
    review: bool = False


@dataclass(frozen=True)
class Link:
    start: int
    end: int
    target: str
    line: int
    kind: str = "markdown"


@dataclass(frozen=True, order=True)
class IncomingLink:
    path: str
    line: int
    target: str
    kind: str
    protected: bool


@dataclass(frozen=True)
class Change:
    path: Path
    before: bytes
    after: bytes


@dataclass
class Plan:
    changes: list[Change]
    findings: list[Finding]
    inputs: dict[Path, bytes] = field(default_factory=dict)


def read_text(path: Path) -> str:
    if path.stat().st_size > MAX_TEXT:
        raise ValueError(f"Text exceeds the {MAX_TEXT}-byte reading limit")
    return path.read_bytes().decode("utf-8")


def mask_code(text: str, *, inline: bool = True) -> str:
    """Keep offsets while hiding fenced/indented code, comments and inline code."""
    parts, opening, list_indents = [], None, []
    fence_indent = 0
    for line in text.splitlines(keepends=True):
        expanded = line.expandtabs(4).rstrip("\r\n")
        indent = len(expanded) - len(expanded.lstrip(" "))
        content = expanded
        base = fence_indent if opening else 0
        if opening is None:
            if expanded.strip():
                while list_indents and indent < list_indents[-1]:
                    list_indents.pop()
            base = list_indents[-1] if list_indents else 0
            marker = re.match(r"^( *)(?:[-+*]|\d{1,9}[.)])( +)", expanded)
            if marker and len(marker[1]) <= base + 3:
                # A list continuation needs four *additional* columns to be code.
                # More than four spaces after a marker means one padding space
                # followed by an indented code block, as in ordinary Markdown.
                base = marker.end() if len(marker[2]) <= 4 else marker.start(2) + 1
                list_indents.append(base)
            content = expanded[base:] if indent >= base or marker else expanded
        elif indent >= base:
            content = expanded[base:]
        fence = FENCE.match(content)
        hidden = opening is not None or bool(fence) or content.startswith("    ")
        if opening is None and fence:
            opening = fence[1]
            fence_indent = base
        elif opening and fence and fence[1][0] == opening[0] and len(fence[1]) >= len(opening) and not fence[2].strip():
            opening = None
        parts.append(re.sub(r"[^\r\n]", " ", line) if hidden else line)
    masked = "".join(parts)
    blank = lambda m: re.sub(r"[^\r\n]", " ", m[0])
    masked = re.sub(r"<!--[\s\S]*?-->", blank, masked)
    return re.sub(r"(`+)(?!`)([\s\S]*?)(?<!`)\1(?!`)", blank, masked) if inline else masked


def destination(text: str, start: int) -> tuple[int, int] | None:
    while start < len(text) and text[start].isspace():
        start += 1
    if start == len(text):
        return None
    if text[start] == "<":
        end = start + 1
        while end < len(text) and text[end] not in ">\n\r":
            end += 2 if text[end] == "\\" else 1
        return (start + 1, end) if end < len(text) and text[end] == ">" else None
    end, depth = start, 0
    while end < len(text):
        char = text[end]
        if char == "\\":
            end += 2
            continue
        if char.isspace() or (char == ")" and depth == 0):
            break
        depth += (char == "(") - (char == ")")
        end += 1
    return (start, end) if depth == 0 else None


def markdown_links(text: str) -> list[Link]:
    masked = mask_code(text)
    spans = {}
    for match in re.finditer(r"(?<!\\)\]\(|^ {0,3}\[(?!\^)[^\]\n]+\]:[ \t]*", masked, re.M):
        span = destination(masked, match.end())
        if span:
            tail = span[1] + (masked[span[0] - 1:span[0]] == "<")
            if match[0] == "](" and not re.match(r'''\s*(?:"[^"\n]*"|'[^'\n]*'|\([^()\n]*\))?\s*\)''', masked[tail:]):
                continue
            spans[span] = "markdown"
    for match in re.finditer(r"<(?:https?://|codex://)[^<>\s]+>", masked):
        spans[(match.start() + 1, match.end() - 1)] = "autolink"
    for match in re.finditer(r"\b(?:href|src)\s*=\s*([\"'])(.*?)\1", masked, re.I):
        spans[match.span(2)] = "html"
    return [Link(start, end, text[start:end], text.count("\n", 0, start) + 1, kind)
            for (start, end), kind in sorted(spans.items())]


def undefined_references(text: str) -> list[int]:
    masked = mask_code(text)
    key = lambda label: " ".join(label.casefold().split())
    definitions = {key(m[1]) for m in re.finditer(r"^ {0,3}\[([^]\n]+)\]:", masked, re.M)}
    return [text.count("\n", 0, match.start()) + 1
            for match in re.finditer(r"(?<![\\\[])\[([^]\n]+)\]\[([^]\n]*)\]", masked)
            if key(match[2] or match[1]) not in definitions]


def cargo_links(text: str) -> list[Link]:
    """Read TOML strings as tokens so comments and quoted examples cannot match."""
    tomllib.loads(text)
    tokens = re.compile(r"\#[^\n]*|\"\"\"[\s\S]*?\"\"\"|'''[\s\S]*?'''|\"(?:\\.|[^\"\\])*\"|'[^']*'")
    links, end = [], 0
    for token in tokens.finditer(text):
        prefix = text[end:token.start()]
        value = token[0]
        if not value.startswith("#") and re.search(r"\bpath\s*=\s*$", prefix):
            if value.startswith(('"""', "'''")):
                raise ValueError("Multiline Cargo path strings need manual review")
            decoded = tomllib.loads("path = " + value)["path"]
            links.append(Link(token.start() + 1, token.end() - 1, decoded,
                              text.count("\n", 0, token.start()) + 1, "cargo"))
        end = token.end()
    return links


def local_target(source: Path, link: Link) -> tuple[Path, str] | None:
    value = html.unescape(re.sub(r"\\([!\"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~])", r"\1", link.target))
    if link.kind == "cargo":
        return absolute(source.parent / value), ""
    parts = urlsplit(value)
    if parts.scheme or parts.netloc:
        return None
    path = absolute(source.parent / unquote(parts.path)) if parts.path else source
    return path, parts.fragment


def inventory(policy: Policy) -> tuple[list[Path], list[Path], list[Finding]]:
    files, repos, findings = set(), set(), []
    def error(exc):
        findings.append(Finding(str(exc.filename), 1, "read-error", str(exc)))
    for root in policy.roots:
        for directory, dirs, names in os.walk(root, followlinks=False, onerror=error):
            path = Path(directory)
            if ".git" in dirs or ".git" in names:
                repos.add(path)
            if path.name == "Evidence" and "Records" in path.parts:
                findings.append(Finding(str(path), 1, "parked-evidence", "Review this Records/Evidence folder with its owner."))
            dirs[:] = sorted(d for d in dirs if d not in SKIP and not (path / d).is_symlink())
            for name in names:
                candidate = path / name
                if not candidate.is_symlink() and (candidate.suffix.lower() == ".md" or name == "Cargo.toml"):
                    files.add(candidate)
    return sorted(files), sorted(repos), findings


def git(repo: Path, *args: str) -> bytes:
    result = subprocess.run(["git", "--no-optional-locks", "-C", str(repo), *args],
                            capture_output=True, timeout=30, check=False)
    if result.returncode:
        raise ValueError("Git could not read " + " ".join(args[:2]))
    return result.stdout


class Moves:
    """Find history and exact-content candidates; every suggestion needs review."""

    def __init__(self, repos: list[Path]):
        self.repos = sorted(repos, key=lambda p: len(p.parts), reverse=True)
        self.history: dict[Path, list[tuple[Path, Path]]] = {}
        self.untracked: dict[Path, list[Path]] = {}
        self.cache: dict[Path, tuple[str, ...]] = {}

    def repository(self, path: Path) -> Path | None:
        return next((repo for repo in self.repos if within(path, repo)), None)

    def candidates(self, missing: Path) -> tuple[str, ...]:
        if missing in self.cache:
            return self.cache[missing]
        repo = self.repository(missing)
        if repo is None:
            return ()
        if repo not in self.history:
            # No pathspec: filtering by the old name hides later rename steps.
            fields = git(repo, "log", "--reverse", "-M", "--diff-filter=R", "--name-status", "--format=", "-z").split(b"\0")
            renames, index = [], 0
            while index < len(fields):
                token = fields[index].strip()
                if re.fullmatch(rb"R\d+", token) and index + 2 < len(fields):
                    renames.append((repo / os.fsdecode(fields[index + 1]), repo / os.fsdecode(fields[index + 2])))
                    index += 3
                else:
                    index += 1
            self.history[repo] = renames
        paths = {missing}
        for old, new in self.history[repo]:
            if old in paths:
                # Retain branches/reused names as ambiguity rather than choosing one.
                paths.add(new)
        found = {p for p in paths if p != missing and p.is_file()
                 and not any(parent.is_symlink() for parent in (p, *p.parents))}
        if not found:
            relative = missing.relative_to(repo).as_posix()
            # Content comparisons are only for ordinary documentation/manifests.
            if missing.suffix.lower() == ".md" or missing.name == "Cargo.toml":
                revisions = ["HEAD"]
                revisions += [rev.decode() + "^" for rev in git(repo, "log", "--format=%H", "--", relative).splitlines()]
                original = None
                for revision in revisions:
                    spec = f"{revision}:{relative}"
                    try:
                        if int(git(repo, "cat-file", "-s", spec)) <= MAX_TEXT:
                            original = git(repo, "show", spec)
                        break
                    except ValueError:
                        continue
                if original is not None:
                    if repo not in self.untracked:
                        self.untracked[repo] = [repo / os.fsdecode(p) for p in git(repo, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0") if p]
                    for candidate in self.untracked[repo]:
                        if (candidate.suffix == missing.suffix and candidate.is_file()
                                and not any(p.is_symlink() for p in (candidate, *candidate.parents))
                                and candidate.stat().st_size == len(original)
                                and candidate.read_bytes() == original):
                            found.add(candidate)
        result = tuple(str(p) for p in sorted(found))
        self.cache[missing] = result
        return result


def metadata(text: str) -> dict[str, str | list[str]]:
    """Read the scalar and string-list subset used by private page properties.

    Other properties are left alone. Unsupported tags/aliases syntax is an error,
    not an empty list that makes checks silently pass. No YAML objects execute.
    """
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        return {}
    try:
        end = lines.index("---", 1)
    except ValueError as exc:
        raise ValueError("Frontmatter has no closing ---") from exc
    result, key = {}, None
    def scalar(value):
        value = re.sub(r"\s+#.*$", "", value.strip())
        if value.startswith('"'):
            return json.loads(value)
        if value.startswith("'") and value.endswith("'"):
            return value[1:-1].replace("''", "'")
        if not value or value[0] in "[{}&*!|>":
            raise ValueError("Unsupported tags or aliases value")
        return value
    for line in lines[1:end]:
        match = re.match(r"^([\w-]+):\s*(.*)$", line)
        if match:
            key, value = match.groups()
            if key in ("tags", "aliases"):
                if not value:
                    result[key] = []
                elif value.startswith("[") and value.endswith("]"):
                    # Commas inside quoted aliases must not divide a value.
                    tokens = re.findall(r'''"(?:\\.|[^"\\])*"|'(?:''|[^'])*'|[^,]+''', value[1:-1])
                    result[key] = [scalar(token) for token in tokens if token.strip()]
                else:
                    raise ValueError(f"{key} must be a list")
            elif key == "type":
                result[key] = scalar(value)
        elif key in ("tags", "aliases") and line.strip() and not line.lstrip().startswith("#"):
            item = re.match(r"^\s*-\s+(.+)$", line)
            if not item:
                raise ValueError(f"Unsupported {key} list entry")
            result[key].append(scalar(item[1]))
    return result


def normalized(value: str) -> str:
    return unicodedata.normalize("NFC", value).casefold().strip()


def topic_table(text: str) -> tuple[str, ...]:
    """Read the Root/Topics table; an absent or ambiguous table is an error."""
    lines = mask_code(text, inline=False).splitlines()
    headers = [i for i, line in enumerate(lines)
               if re.fullmatch(r"\s*\|\s*Root\s*\|\s*Topics\s*\|\s*", line)]
    if len(headers) != 1:
        raise ValueError("Supply a standard with exactly one Root/Topics table.")
    index = headers[0] + 1
    if index >= len(lines) or not re.fullmatch(r"\s*\|[\s:|-]+\|\s*", lines[index]):
        raise ValueError("The topic table needs a Markdown separator row.")
    roots = []
    for line in lines[index + 1:]:
        if not line.lstrip().startswith("|"):
            break
        row = re.fullmatch(r"\s*\|\s*`([a-z][a-z0-9-]*)`\s*\|[^|]+\|\s*", line)
        if not row or row[1] in roots:
            raise ValueError("Topic roots must be unique lowercase names in backticks.")
        roots.append(row[1])
    if not roots:
        raise ValueError("The topic table is empty.")
    return tuple(roots)


def node_for(path: Path, policy: Policy, pages: dict[Path, str]) -> Path | None:
    explicit = [p for p in policy.nodes if within(path, p)]
    if explicit:
        return max(explicit, key=lambda p: len(p.parts))
    for parent in path.parents:
        if parent.name in {"Docs", "Content", "Creative"} and (parent.parent / "README.md") in pages:
            return parent.parent
    for parent in path.parents:
        if not policy.private(parent):
            break
        if (parent / "README.md") in pages:
            return parent
    return None


def subject_review(path: Path, policy: Policy, pages: dict[Path, str]) -> Finding | None:
    """A physical node suggests a subject; it cannot settle a page's ownership."""
    node = node_for(path, policy, pages)
    if node is None:
        return None
    tags = metadata(pages[path]).get("tags", [])
    root_tags = metadata(pages.get(node / "README.md", "")).get("tags", [])
    expected = policy.nodes.get(node) or (root_tags[0] if root_tags else None)
    if expected and (not tags or tags[0] != expected):
        return Finding(str(path), 1, "subject-tag",
                       f"Review subject {tags[0] if tags else '(missing)'} against node {expected}; "
                       "a page may intentionally describe another subject.", review=True)
    return None


def graph_plan(policy: Policy) -> Plan:
    """Regenerate only simple topic groups, preserving colours and custom queries."""
    plan = Plan([], [])
    if policy.topic_standard is not None:
        plan.inputs[policy.topic_standard] = policy.topic_standard.read_bytes()
        roots = topic_table(plan.inputs[policy.topic_standard].decode("utf-8"))
    else:
        roots = policy.topic_roots
    for path in policy.graphs:
        try:
            before = read_text(path)
            graph = json.loads(before)
            groups = graph.get("colorGroups", [])
            if not isinstance(groups, list) or any(not isinstance(g, dict) for g in groups):
                raise ValueError("Graph colorGroups must be a list of objects.")
            changed = False
            for group in groups:
                query = group.get("query", "")
                match = re.fullmatch(r"tag:#([a-z][a-z0-9-]*)(?: -tag:#[a-z][a-z0-9-]*)*", query)
                if match and match[1] in roots:
                    # Unknown exclusions may be personal filters, not stale topics.
                    extra = [term for term in query.split()[1:]
                             if term.removeprefix("-tag:#") not in roots
                             or term.removeprefix("-tag:#") == match[1]]
                    derived = "tag:#" + match[1] + "".join(" -tag:#" + r for r in roots if r != match[1])
                    derived += "".join(" " + term for term in extra)
                    if query != derived:
                        group["query"], changed = derived, True
            if changed:
                after = json.dumps(graph, ensure_ascii=False, indent=2) + ("\n" if before.endswith("\n") else "")
                plan.changes.append(Change(path, before.encode("utf-8"), after.encode("utf-8")))
        except (OSError, ValueError, TypeError, AttributeError) as exc:
            plan.findings.append(Finding(str(path), 1, "graph-topics", str(exc)))
    return plan


def set_subject(text: str, subject: str, topics: tuple[str, ...]) -> str:
    """Change the reviewed subject alone; retain all topics, manual tags and prose."""
    if not re.fullmatch(r"[a-z][a-z0-9-]*(?:/[a-z0-9-]+)*", subject) or subject.split("/")[0] in topics:
        raise ValueError("Supply a lowercase subject tag, not a topic root.")
    tags = metadata(text).get("tags", [])
    if tags and tags[0] == subject:
        return text
    lines = text.splitlines(keepends=True)
    if not lines or lines[0].strip() != "---":
        raise ValueError("Subject repairs require existing YAML frontmatter.")
    end = next(i for i in range(1, len(lines)) if lines[i].strip() == "---")
    index = next((i for i in range(1, end) if lines[i].startswith("tags:")), None)
    newline = "\r\n" if lines[0].endswith("\r\n") else "\n"
    insert = not tags or tags[0].split("/")[0] in topics
    if index is None:
        lines.insert(end, "tags:" + newline + "  - " + subject + newline)
    elif lines[index][5:].strip():
        selected = [subject, *tags] if insert else [subject, *tags[1:]]
        lines[index] = "tags: " + json.dumps(selected, ensure_ascii=False) + newline
    elif insert:
        lines.insert(index + 1, "  - " + subject + newline)
    else:
        first = next(i for i in range(index + 1, end) if re.match(r"\s*-\s+", lines[i]))
        match = re.fullmatch(r"(\s*-\s+)(.*?)(\s+#.*)?(\r?\n)?", lines[first])
        lines[first] = match[1] + subject + (match[3] or "") + (match[4] or "")
    return "".join(lines)


def plan_tags(policy: Policy, subjects: dict[Path, str] | None = None) -> Plan:
    """Preview graph updates and explicit PAGE=TAG choices; never infer a rewrite.

    An explicit choice may retain an intentional cross-folder subject. It closes
    that review for this run without creating another exceptions register.
    """
    if policy.topic_standard is None:
        raise ValueError("Tag repair requires --topic-standard.")
    selected = {absolute(p): tag for p, tag in (subjects or {}).items()}
    plan, pages = graph_plan(policy), {}
    topics = topic_table(plan.inputs[policy.topic_standard].decode("utf-8"))
    files, _, _ = inventory(policy)
    for path in files:
        if path.suffix.lower() != ".md" or not policy.living(path) or any(within(path, p) for p in policy.protected):
            continue
        try:
            text = read_text(path)
            # Original prompts and publication source have their own conventions.
            if metadata(text):
                pages[path] = text
        except (OSError, ValueError, UnicodeError) as exc:
            plan.findings.append(Finding(str(path), 1, "frontmatter", str(exc)))
    for path, text in pages.items():
        if path not in selected:
            finding = subject_review(path, policy, pages)
            if finding:
                plan.findings.append(finding)
    for path, subject in selected.items():
        if path not in pages or not policy.can_write(path):
            plan.findings.append(Finding(str(path), 1, "protected-repair", "Select a writable living page with frontmatter."))
            continue
        try:
            after = set_subject(pages[path], subject, topics)
            if after != pages[path]:
                plan.changes.append(Change(path, pages[path].encode("utf-8"), after.encode("utf-8")))
        except ValueError as exc:
            plan.findings.append(Finding(str(path), 1, "subject-tag", str(exc)))
    return plan


def google_key(url: str) -> str:
    parts = urlsplit(html.unescape(url))
    match = re.search(r"/(?:d|folders)/([^/]+)", parts.path)
    if match:
        return match[1]
    from urllib.parse import parse_qs
    return parse_qs(parts.query).get("id", [parts.netloc + parts.path])[0]


def heading_ids(text: str) -> set[str]:
    """Recognize ordinary GitHub headings, duplicate slugs, explicit HTML IDs and the
    `{#id}` heading attribute that Gazette and other Markdown renderers honour."""
    masked = mask_code(text, inline=False)
    result = set(re.findall(r'<[A-Za-z][^>]*?\bid\s*=\s*["\']([^"\']+)["\']', mask_code(text)))
    counts = defaultdict(int)
    lines = masked.splitlines()
    for index, line in enumerate(lines):
        heading = re.match(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$", line)
        value = heading[1] if heading else None
        if not heading and index + 1 < len(lines) and re.fullmatch(r" {0,3}(?:=+|-+)\s*", lines[index + 1]) and line.strip():
            value = line.strip()
        if value is None:
            continue
        attribute = re.search(r"\s*\{#([^}\s]+)\}\s*$", value)
        if attribute:
            result.add(attribute[1])
            value = value[:attribute.start()]
        value = re.sub(r"\[([^]]+)\]\([^)]*\)", r"\1", value)
        value = re.sub(r"<[^>]+>", "", html.unescape(value)).lower()
        slug = "".join(c for c in value if c in " -_" or unicodedata.category(c)[0] in "LN")
        slug = slug.replace(" ", "-")
        number = counts[slug]
        counts[slug] += 1
        result.add(slug + (f"-{number}" if number else ""))
    return result


def private_checks(policy: Policy, pages: dict[Path, str], links: dict[Path, list[Link]]) -> list[Finding]:
    findings, properties, aliases, names, google = [], {}, defaultdict(set), defaultdict(set), defaultdict(dict)
    def add(path, line, code, message):
        findings.append(Finding(str(path), line, code, message))
    living = {p: text for p, text in pages.items() if policy.living(p)}
    for path, text in living.items():
        try:
            properties[path] = metadata(text)
        except (ValueError, TypeError) as exc:
            add(path, 1, "frontmatter", str(exc))
            properties[path] = {}
        names[normalized(path.stem)].add(path)
        for alias in properties[path].get("aliases", []):
            aliases[normalized(alias)].add(path)
        for match in GOOGLE.finditer(mask_code(text)):
            google[google_key(match[0])][path] = text.count("\n", 0, match.start()) + 1
        for match in PRIVATE_PATH.finditer(text):
            add(path, text.count("\n", 0, match.start()) + 1, "absolute-path", "Use a relative link or a workspace-supplied path.")
        if re.search(r"\[\[(?!site:)", mask_code(text)):
            add(path, 1, "wiki-link", "Use a Markdown link with a visible label.")
        for link in links.get(path, []):
            target = local_target(path, link)
            if link.target.startswith("codex://") or (target and any(within(target[0], root) for root in policy.working_areas)):
                add(path, link.line, "working-link", "Put the useful account on its owning page before retiring this working material.")
            if target and target[1]:
                add(path, link.line, "private-heading-link", "Link to the page and name the section in prose.")
            line = text.splitlines()[link.line - 1]
            if target and "Records" in target[0].parts and re.search(r"\b(reason|reasoning|rationale|explanation)\b", line, re.I):
                add(path, link.line, "reason-in-records", "Review whether this page sends its explanation into Records.")
    for alias, owners in aliases.items():
        colliding = owners | names.get(alias, set())
        if len(colliding) > 1:
            for path in sorted(owners):
                add(path, 1, "shared-alias", "Alias is also used by: " + ", ".join(str(p) for p in sorted(colliding - {path})))
    for key, uses in google.items():
        home = policy.source_homes.get(key)
        for path, line in uses.items():
            if (home and path != home) or (not home and len(uses) > 1):
                add(path, line, "source-home", "Link to the owning page for this Google source" + (f": {home}" if home else "; its address occurs on several pages."))
    graph = {p: {target[0] for link in links.get(p, []) if (target := local_target(p, link))} for p in living}
    nodes = {p: node_for(p, policy, pages) for p in living}
    reached = {}
    for node in set(nodes.values()) - {None}:
        pending, seen = [node / "README.md"], set()
        while pending:
            current = pending.pop()
            if current in seen:
                continue
            seen.add(current)
            pending.extend(p for p in graph.get(current, ()) if nodes.get(p) == node)
        reached[node] = seen
    for path, text in living.items():
        node = nodes[path]
        if node is None:
            add(path, 1, "missing-readme", "No owning README was found.")
            continue
        readme = node / "README.md"
        if path != readme and path not in reached[node]:
            add(path, 1, "unreachable", "No route from the owning README reaches this page.")
        nearest = next((p / "README.md" for p in path.parents if p / "README.md" in pages and p / "README.md" != path), None)
        routes = graph.get(path, set())
        last = next(((number, line) for number, line in reversed(list(enumerate(text.splitlines(), 1))) if line.strip()), None)
        if last and re.match(r"(?:Part of|Back to)\b", last[1]):
            routes = {target[0] for link in links.get(path, [])
                      if link.line == last[0] and (target := local_target(path, link))}
        if nearest and not (nearest in routes or any(
                nodes.get(target) == node and nearest in graph.get(target, set())
                for target in routes)):
            add(path, len(text.splitlines()), "parent-route", "Link to the nearest README or a same-node hub that links back to it.")
        tags = properties[path].get("tags", [])
        try:
            finding = subject_review(path, policy, pages)
            if finding:
                findings.append(finding)
        except ValueError:
            pass  # The frontmatter finding above already reports this error.
        if len(tags) > 3:
            add(path, 1, "topic-tags", "Keep the subject tag and at most two topic tags.")
        if policy.topic_roots and any(tag.split("/")[0] not in policy.topic_roots for tag in tags[1:]):
            add(path, 1, "topic-tags", "A topic tag uses a root outside the workspace vocabulary.")
    return findings


def check(policy: Policy, *, suggest_moves: bool = True) -> list[Finding]:
    """Check both trees with one inventory; return page/line findings, never write."""
    files, repos, findings = inventory(policy)
    moves, pages, links = Moves(repos), {}, {}
    anchors = {}
    for path in files:
        try:
            text = read_text(path)
            if policy.sealed(path):
                if policy.original(path) and path.name == "README.md" and re.search(r"\b[0-9a-fA-F]{32,}\b|\b(?:inode|xattr|com\.apple\.\w+)\s*[:=]\s*\S+", text):
                    findings.append(Finding(str(path), 1, "record-metadata", "Review hashes, inode or filesystem attributes in this Records README."))
                continue
            parsed = cargo_links(text) if path.name == "Cargo.toml" else markdown_links(text)
            for link in parsed:
                local_target(path, link)
            links[path] = parsed
            if path.suffix.lower() == ".md":
                pages[path] = text
                findings.extend(Finding(str(path), line, "missing-reference", "Markdown reference has no definition.")
                                for line in undefined_references(text))
            public_root = policy.publication_root(path)
            if public_root is not None:
                for match in PRIVATE_PATH.finditer(text):
                    findings.append(Finding(str(path), text.count("\n", 0, match.start()) + 1, "public-private-path", "Repository text contains a private machine path."))
            for link in parsed:
                target = local_target(path, link)
                if target is None:
                    continue
                destination_path, fragment = target
                if public_root is not None and not within(destination_path, public_root):
                    findings.append(Finding(str(path), link.line, "public-local-link", "Repository links must stay in their repository or use a public address."))
                if not destination_path.exists():
                    candidates = ()
                    if suggest_moves:
                        try:
                            candidates = moves.candidates(destination_path)
                        except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
                            findings.append(Finding(str(path), link.line, "move-search-error", str(exc)))
                    findings.append(Finding(str(path), link.line, "missing-target", f"Target does not exist: {destination_path}", candidates, str(destination_path)))
                elif fragment and destination_path.suffix.lower() == ".md" and not policy.original(destination_path):
                    if destination_path not in anchors:
                        anchors[destination_path] = heading_ids(read_text(destination_path))
                    if unquote(fragment) not in anchors[destination_path]:
                        findings.append(Finding(str(path), link.line, "missing-heading", f"Heading does not exist in {destination_path}: #{fragment}"))
        except (OSError, ValueError, UnicodeError, subprocess.TimeoutExpired) as exc:
            findings.append(Finding(str(path), 1, "read-error", str(exc)))
    findings.extend(private_checks(policy, pages, links))
    graph = graph_plan(policy)
    findings.extend(graph.findings)
    findings.extend(Finding(str(c.path), 1, "graph-topics", "Regenerate topic queries from the supplied standard with tags --apply.")
                    for c in graph.changes)
    for task, ended in policy.task_ends.items():
        if ended < policy.today and task.exists():
            findings.append(Finding(str(task), 1, "ended-task-output", f"Working material remains after the declared task end ({ended}); review with its owner."))
    for root in policy.build_roots:
        findings.extend(build_findings(measure(root, policy.pale_executable), policy.build_limit))
    for hook in policy.hooks:
        try:
            for number, line in enumerate(read_text(hook).splitlines(), 1):
                for command in policy.retired_commands:
                    if command in line:
                        findings.append(Finding(str(hook), number, "retired-hook", f"Hook still calls the retired {command}."))
        except (OSError, ValueError, UnicodeError) as exc:
            findings.append(Finding(str(hook), 1, "read-error", str(exc)))
    return sorted(set(findings))


def incoming_links(policy: Policy, target: Path) -> tuple[list[IncomingLink], list[Finding]]:
    """List references from other files, including protected originals, without writes.

    Match an exact local destination even when it no longer exists. Queries and
    fragments do not change the page identity. Reference-style Markdown reports
    its destination definition, not each use of that definition.
    """
    target = absolute(target)
    if not any(within(target, root) for root in policy.roots):
        raise ValueError("The incoming-link target must be inside a declared search root.")
    files, _, notices = inventory(policy)
    findings = [f for f in notices if f.code == "read-error"]
    incoming = []
    for path in files:
        if path == target:
            continue
        try:
            text = read_text(path)
            links = cargo_links(text) if path.name == "Cargo.toml" else markdown_links(text)
            protected = policy.original(path) or any(within(path, root) for root in policy.protected)
            for link in links:
                destination = local_target(path, link)
                if destination is not None and destination[0] == target:
                    incoming.append(IncomingLink(str(path), link.line, link.target, link.kind, protected))
        except (OSError, ValueError, UnicodeError) as exc:
            findings.append(Finding(str(path), 1, "read-error", str(exc)))
    return sorted(set(incoming)), sorted(set(findings))


def build_findings(report: BuildSize, limit: int | None) -> list[Finding]:
    """Apply the same failure and size rules to checks and standalone reports."""
    findings = [Finding(report.root, 1, "build-read-error", message) for message in report.errors]
    if limit is not None and report.logical_bytes is not None and report.logical_bytes > limit:
        qualifier = "" if report.complete else "at least "
        findings.append(Finding(report.root, 1, "build-size",
                                f"Build tree is {qualifier}{report.logical_bytes} logical bytes; the declared limit is {limit}."))
    return findings


def remap(path: Path, old: Path, new: Path) -> Path:
    return new / path.relative_to(old) if within(path, old) else path


def plan_relink(policy: Policy, old: Path, new: Path) -> Plan:
    """Plan inbound and moved-file-relative repairs after a move, across all roots.

    old must be absent and new must exist. The caller selects the move explicitly;
    neither missing targets nor Git candidates authorize a move. Any changed
    protected file blocks applying the whole plan, with its location reported.
    """
    old, new = absolute(old), absolute(new)
    if old == new or within(new, old) or within(old, new):
        raise ValueError("Move paths must be distinct and must not contain each other.")
    if old.exists() or not new.exists():
        raise ValueError("Run relink after the move: OLD must be absent and NEW must exist.")
    if not all(any(within(p, root) for root in policy.roots) for p in (old, new)):
        raise ValueError("Both move paths must be inside the declared search roots.")
    if not policy.can_write(new):
        raise ValueError("The moved destination is not in a writable, unprotected root.")
    files, _, findings = inventory(policy)
    # Inventory review notices do not prevent a repair; read failures do.
    findings = [f for f in findings if f.code == "read-error"]
    changes = []
    for path in files:
        try:
            text = read_text(path)
            parsed = cargo_links(text) if path.name == "Cargo.toml" else markdown_links(text)
            previous = remap(path, new, old)
            replacements = []
            for link in parsed:
                target = local_target(previous, link)
                if target is None:
                    continue
                destination_path, _ = target
                desired = remap(destination_path, old, new)
                current = local_target(path, link)[0]
                if current == desired:
                    continue
                relative = os.path.relpath(desired, path.parent).replace(os.sep, "/")
                if link.kind == "cargo":
                    value = relative if text[link.start - 1] == "'" else json.dumps(relative, ensure_ascii=False)[1:-1]
                    if "'" in value and text[link.start - 1] == "'":
                        raise ValueError("A literal Cargo path needs a quote change; review it manually")
                else:
                    suffix = re.search(r"[?#].*$", link.target)
                    value = quote(relative, safe="/.-_~") + (html.unescape(suffix[0]) if suffix else "")
                    if link.kind == "html":
                        value = html.escape(value, quote=True)
                replacements.append((link.start, link.end, value))
            if replacements:
                if not policy.can_write(path):
                    findings.append(Finding(str(path), 1, "protected-repair", "This link needs repair, but the file is outside writable roots or is original/protected material."))
                    continue
                updated = text
                for start, end, value in reversed(replacements):
                    updated = updated[:start] + value + updated[end:]
                changes.append(Change(path, text.encode(), updated.encode()))
        except (OSError, ValueError, UnicodeError) as exc:
            findings.append(Finding(str(path), 1, "read-error", str(exc)))
    return Plan(changes, sorted(findings))


def apply_relink(policy: Policy, plan: Plan) -> None:
    """Preflight every input, then atomically replace each file with its plan.

    No cross-file transaction is claimed: an I/O failure after a replacement may
    leave a partial repair. The raised error names the completed files. Callers
    should review the ordinary repository diff before another attempt.
    """
    if any(not finding.review for finding in plan.findings):
        raise ValueError("Resolve the reported repair problems before applying.")
    for path, before in plan.inputs.items():
        if path.read_bytes() != before:
            raise ValueError(f"Repair input changed: {path}")
    for change in plan.changes:
        if not policy.can_write(change.path) or change.path.read_bytes() != change.before:
            raise ValueError(f"File changed or is no longer writable: {change.path}")
    completed = []
    try:
        for change in plan.changes:
            mode = stat.S_IMODE(change.path.stat().st_mode)
            fd, temporary = tempfile.mkstemp(prefix=".vault-relink-", dir=change.path.parent)
            try:
                with os.fdopen(fd, "wb") as output:
                    output.write(change.after)
                    output.flush()
                    os.fsync(output.fileno())
                os.chmod(temporary, mode)
                os.replace(temporary, change.path)
                completed.append(str(change.path))
            finally:
                if os.path.exists(temporary):
                    os.unlink(temporary)
    except OSError as exc:
        raise ValueError(f"Repair stopped; completed files: {completed}. {exc}") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", action="append", type=Path, required=True, help="Search tree; repeat for knowledge and repositories")
    parser.add_argument("--vault", action="append", type=Path, default=[], help="Private knowledge root receiving page checks")
    parser.add_argument("--public-root", action="append", type=Path, default=[], help="Tree intended for publication; Git remotes do not select this scope")
    parser.add_argument("--writable", action="append", type=Path, default=[], help="Root explicitly allowed to receive repairs")
    parser.add_argument("--protect", action="append", type=Path, default=[], help="Client, upstream or sealed root that must remain unchanged")
    parser.add_argument("--original-name", action="append", default=[], metavar="NAME", help="Folder name that holds original material, in addition to Records and Sources")
    parser.add_argument("--working-area", action="append", type=Path, default=[])
    parser.add_argument("--task-end", action="append", default=[], metavar="PATH=YYYY-MM-DD", help="Declared end of a working folder's task; never inferred from file age")
    parser.add_argument("--hook", action="append", type=Path, default=[])
    parser.add_argument("--retired-command", action="append", default=[], metavar="NAME", help="Executable a hook must no longer call; repeat for each")
    parser.add_argument("--build-root", action="append", type=Path, default=[], help="Build directory to measure; repeat for separate trees")
    parser.add_argument("--pale-executable", type=Path, help="Absolute native Pale executable used for build measurement")
    parser.add_argument("--build-limit", type=int, help="Maximum logical regular-file bytes in each declared build tree")
    parser.add_argument("--topic-standard", type=Path, help="Markdown standard holding the Root/Topics table")
    parser.add_argument("--graph", action="append", type=Path, default=[], help="Obsidian graph settings; keep colours and personal settings")
    parser.add_argument("--json", action="store_true")
    commands = parser.add_subparsers(dest="command", required=True)
    checker = commands.add_parser("check", help="Report findings without changing files")
    checker.add_argument("--no-move-suggestions", action="store_true")
    incoming = commands.add_parser("incoming", help="List links into a local page or path without changing files")
    incoming.add_argument("target", type=Path, help="Exact destination, including a former path that no longer exists")
    commands.add_parser("builds", help="Report every declared build tree, including totals below the limit")
    relinker = commands.add_parser("relink", help="Preview repairs after OLD has moved to NEW")
    relinker.add_argument("old", type=Path)
    relinker.add_argument("new", type=Path)
    relinker.add_argument("--apply", action="store_true")
    tagger = commands.add_parser("tags", help="Preview derived graph queries and reviewed subject choices")
    tagger.add_argument("--subject", action="append", default=[], metavar="PAGE=TAG", help="Reviewed subject to write or retain; never inferred from the physical parent")
    tagger.add_argument("--apply", action="store_true")
    args = parser.parse_args(argv)
    try:
        policy = Policy(tuple(args.root), tuple(args.vault), tuple(args.writable), tuple(args.protect),
                        working_areas=tuple(args.working_area), hooks=tuple(args.hook), retired_commands=tuple(args.retired_command),
                        original_names=tuple(args.original_name),
                        public_roots=tuple(args.public_root),
                        topic_standard=args.topic_standard, graphs=tuple(args.graph),
                        build_roots=tuple(args.build_root), build_limit=args.build_limit, pale_executable=args.pale_executable,
                        task_ends={Path(item.rsplit("=", 1)[0]): date.fromisoformat(item.rsplit("=", 1)[-1]) for item in args.task_end})
        changes = []
        reports = []
        backlinks = []
        if args.command == "check":
            findings = check(policy, suggest_moves=not args.no_move_suggestions)
        elif args.command == "incoming":
            backlinks, findings = incoming_links(policy, args.target)
        elif args.command == "builds":
            if not policy.build_roots:
                raise ValueError("Supply at least one build directory with --build-root.")
            reports = [measure(root, policy.pale_executable) for root in policy.build_roots]
            findings = [finding for report in reports for finding in build_findings(report, policy.build_limit)]
        else:
            if args.command == "tags":
                if any("=" not in item for item in args.subject):
                    raise ValueError("Use --subject PAGE=TAG for each reviewed page.")
                plan = plan_tags(policy, {Path(item.rsplit("=", 1)[0]): item.rsplit("=", 1)[1] for item in args.subject})
            else:
                plan = plan_relink(policy, args.old, args.new)
            findings, changes = plan.findings, [str(c.path) for c in plan.changes]
            if args.apply:
                apply_relink(policy, plan)
        if args.json:
            output = {"findings": [asdict(f) for f in findings], "changes": changes}
            if args.command == "incoming":
                output["target"] = str(absolute(args.target))
                output["incoming"] = [asdict(link) for link in backlinks]
            if args.command == "builds":
                output["builds"] = [asdict(report) for report in reports]
            print(json.dumps(output, indent=2))
        else:
            if args.command == "incoming":
                for link in backlinks:
                    protection = " [protected]" if link.protected else ""
                    print(f"{link.path}:{link.line}:{protection} {link.target}")
                print(f"{len(backlinks)} incoming links in the selected roots" +
                      ("; incomplete scan" if findings else ""))
            for report in reports:
                amount = "unavailable" if report.logical_bytes is None else f"{report.logical_bytes} logical bytes"
                status = "complete" if report.complete else "incomplete"
                print(f"{report.root}: {amount}; {report.file_count} files; {report.skipped_symlinks} symlinks skipped; {status}")
            for finding in findings:
                print(f"{finding.path}:{finding.line}: {'review' if finding.review else 'error'}: {finding.code}: {finding.message}")
                for candidate in finding.candidates:
                    print(f"  Possible move, review before relinking: {candidate}")
            for path in changes:
                print(f"{'Updated' if args.apply else 'Would update'}: {path}")
        return 1 if any(not finding.review for finding in findings) else 0
    except (OSError, ValueError) as exc:
        print(str(exc), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
