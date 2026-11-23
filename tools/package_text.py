#!/usr/bin/env python3
"""Warn about hard-to-read Package prose and check material carried into a port."""

import argparse
import json
from pathlib import Path
import re


FENCE = re.compile(r"^\s{0,3}(`{3,}|~{3,})(.*)$")
INLINE = re.compile(r"(`+)(.+?)\1")
LINK = re.compile(r"\]\((?:<([^>]+)>|([^\s)]+))\)|^\[[^]]+\]:\s*<?([^\s>]+)", re.M)
NUMBER = re.compile(r"(?<![\w-])\d+(?:[.,]\d+)*(?:%|ms|s|px|rem|vh|fps)?\b%?")
WORD = re.compile(r"\b[\w]+(?:['’-][\w]+)*\b")
RULE = re.compile(r"\b(?:must|never|only)\b", re.I)


def markdown_parts(text):
    prose, code = [], []
    opening = None
    for line in text.splitlines():
        match = FENCE.match(line)
        if opening is None:
            if match:
                opening = match[1]
                block = []
            else:
                prose.append(line)
        elif match and match[1][0] == opening[0] and len(match[1]) >= len(opening) and not match[2].strip():
            code.append("\n".join(block).strip())
            opening = None
        else:
            block.append(line)
    if opening is not None:
        raise ValueError("Unclosed code fence")
    return prose, code


def prose_lines(text):
    return markdown_parts(text)[0]


def plain(line):
    return LINK.sub("]", INLINE.sub("", line))


def sentences(lines):
    chunk = ""
    for raw in lines:
        line = raw.strip()
        if not line or line.startswith(("#", "|", "- ", "* ", ">")):
            if chunk:
                yield from re.split(r"(?<=[.!?])\s+", chunk)
            chunk = line
        else:
            chunk = f"{chunk} {line}".strip()
    if chunk:
        yield from re.split(r"(?<=[.!?])\s+", chunk)


def word_pattern(text):
    """Match caller-supplied words; a final * includes word continuations."""
    patterns = []
    for number, line in enumerate(text.splitlines(), 1):
        term = line.strip()
        if not term:
            continue
        stem = term.removesuffix("*")
        if not WORD.fullmatch(stem):
            raise ValueError(f"Word list line {number}: use one word, optionally ending in *")
        suffix = r"[\w'’-]*" if term.endswith("*") else ""
        patterns.append(re.escape(stem) + suffix)
    if not patterns:
        raise ValueError("Word list must contain at least one word")
    return re.compile(r"\b(?:" + "|".join(patterns) + r")\b", re.I)


def register(text, vocabulary=None):
    lines = [plain(line) for line in prose_lines(text)]
    flagged = sum(len(vocabulary.findall(line)) for line in lines) if vocabulary else 0
    words = len(WORD.findall(" ".join(lines)))
    dense = [s for s in sentences(lines) if len(vocabulary.findall(s)) >= 2] if vocabulary else []
    return {"words": words, "flagged": flagged, "density": round(100 * flagged / max(words, 1), 3),
            "dense_sentences": dense}


def register_rows(root, vocabulary=None):
    return {
        p.relative_to(root).as_posix(): register(p.read_text(), vocabulary)
        for p in sorted((root / "packages").glob("**/*.md"))
        if p.name != "INDEX.md"
    }


def regressions(current, prior):
    for path, row in current.items():
        old = prior.get(path)
        if old and (row["flagged"] * max(old["words"], 1) >
                    old["flagged"] * max(row["words"], 1) or
                    len(row["dense_sentences"]) > len(old["dense_sentences"])):
            yield path


def features(text):
    lines, blocks = markdown_parts(text)
    prose = "\n".join(lines)
    identifiers = set()
    for _, span in INLINE.findall(prose):
        identifiers.update(re.findall(r"[\w][\w.:/@<>*-]*", span))
    targets = {next(part for part in match if part) for match in LINK.findall(prose)}
    return {
        "numbers": set(NUMBER.findall(LINK.sub("]", prose))),
        "identifiers": identifiers,
        "link_targets": targets,
        "code_blocks": set(blocks),
        "rules": {" ".join(s.split()) for s in sentences(lines) if RULE.search(plain(s))},
    }


def compare(source, destination):
    old, new = features(source), features(destination)
    old_words, new_words = len(source.split()), len(destination.split())
    return {
        "source_words": old_words,
        "new_words": new_words,
        "drop_over_20_percent": new_words * 5 < old_words * 4,
        "missing": {key: sorted(items - new[key]) for key, items in old.items()},
    }


def port_report(mapping, root):
    rows = []
    for row in mapping:
        source = Path(row["source"]).read_text()
        destination = "\n\n".join((root / path).read_text() for path in row["destinations"])
        result = compare(source, destination)
        reasons = row.get("reasons", {})
        flags = {key for key, missing in result["missing"].items() if missing}
        if result["drop_over_20_percent"]:
            flags.add("word_count")
        result.update({
            "source": row["source"],
            "destinations": row["destinations"],
            "reasons": reasons,
            "unexplained": sorted(flags - {key for key, reason in reasons.items() if reason.strip()}),
        })
        rows.append(result)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--baseline", type=Path, help="Compare wording counts as reading prompts, never a pass/fail score")
    parser.add_argument("--word-list", type=Path, help="Caller-owned UTF-8 list: one word per line; final * matches continuations")
    parser.add_argument("--port-map", type=Path)
    parser.add_argument("--warn-words", type=int, default=2200)
    args = parser.parse_args()
    if args.port_map and (args.word_list or args.baseline):
        parser.error("--port-map cannot be combined with --word-list or --baseline")
    if args.port_map:
        rows = port_report(json.loads(args.port_map.read_text()), args.root)
        print(json.dumps(rows, indent=2))
        return int(any(row["unexplained"] for row in rows))
    if args.baseline and not args.word_list:
        parser.error("--baseline requires --word-list for the comparison")
    vocabulary = None
    if args.word_list:
        try:
            vocabulary = word_pattern(args.word_list.read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
            parser.error(str(error))
    current = register_rows(args.root, vocabulary)
    prior = register_rows(args.baseline, vocabulary) if args.baseline else {}
    warnings = sorted(regressions(current, prior))
    print(json.dumps({
        "word_list_configured": vocabulary is not None,
        "files": current,
        "comparison_warnings": warnings,
        "size_warnings": [p for p, row in current.items() if row["words"] > args.warn_words],
    }, indent=2))
    # A count can point a reviewer to prose; it cannot establish whether the
    # rewrite preserved its meaning or reads better.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
