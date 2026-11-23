#!/usr/bin/env python3
"""Static floor check for built HTML and CSS and for a theme's templates, stylesheets and scripts.
Standard library only; no browser.

Usage: static_audit.py [--authored-source FILE] PATH [PATH ...]   (files or directories; scans .html, .htm, .css, .scss, .sass,
                                          .js, .mjs, .ts)
       static_audit.py -h | --help
Exit:  0 no failures · 1 failures · 2 nothing scanned (reported as SKIPPED, never a pass)

--authored-source FILE may be repeated for exact scene-only scripts/styles in the scan.
It excludes the whole file from shared-token checks and reports the exclusion. Page semantics
and remaining checks still run. Scan mixed interface/scene files normally and review findings
by role, or separate their roles before using the exclusion. The scanner cannot infer roles.

Per-file checks
  placeholder-link   <a> with href="#" or an empty href (native links need destinations)
  zoom-lock          viewport meta with user-scalable=no/0 or maximum-scale below 2
  tab-order          positive tabindex (order must come from the DOM)
  img-alt            <img> without an alt attribute (use alt="" for decoration)
  fixed-width        width/min-width above --fixed-width-limit outside a minimum-width query
Optional theme-source checks (--token-roles and --template-emoji; comments ignored; files under a tokens, dist, vendor or node_modules folder of
the scanned path are pinned or third-party output and are skipped)
  literal-colour     hex, rgb(), rgba(), hsl(), hsla(), hwb(), lab(), lch(), oklab(), oklch() or color()
                     value in a declaration, custom property, Sass variable or style attribute; colour
                     functions that read var() or use relative "from" syntax are not flagged
  literal-duration   non-zero ms or s value in a transition or animation declaration (a var() fallback
                     counts; multiplying by one unit to convert a unitless token does not)
  literal-length     non-zero absolute or font-relative length (px, rem, em, ch, ex, lh, rlh, pt, pc,
                     in, cm, mm, Q) in a size property: width, height, inline-size, block-size and their
                     min-/max- forms, margin*, padding*, gap, row-gap, column-gap, inset*, top, right,
                     bottom, left, font-size, line-height, flex-basis, border-radius*, outline-offset,
                     grid-template-* and grid-auto-* (a var() fallback counts; percentages, fr, viewport
                     and container units, zero, keywords, custom properties and Sass variables pass;
                     a value fixed-width already reports is not reported twice)
  literal-motion     in .js, .mjs and .ts: a non-zero number literal as the value of duration, delay,
                     endDelay, loopDelay, visualDuration, bounce, stiffness, damping or mass, or as the
                     first argument of stagger(); a number array or quoted string as the value of ease
                     or easing; a ms or s literal in a string assigned to style.transition* or
                     style.animation* (comments ignored; observer threshold and rootMargin are
                     behavioural constants and pass)
  emoji-icon         emoji in template markup: an Extended_Pictographic character outside the Basic
                     Multilingual Plane, one with default emoji presentation, one followed by U+FE0F,
                     or a regional-indicator flag (Unicode 17.0); text-presentation symbols such as the
                     copyright sign, trademark sign and arrows are typography and pass. A template is an
                     .html or .htm file under a templates folder or containing Tera delimiters.
Site-wide checks (all scanned CSS, including <style> blocks, taken together)
  reduced-motion     animation, transition or smooth scrolling with no prefers-reduced-motion rule
  system-dark        explicit dark-mode selectors (data-mode="dark") with no prefers-color-scheme: dark rule
  focus-removed      outline removed somewhere with no :focus-visible rule anywhere

Passing proves nothing about rendering, contrast, layout or behaviour; those need a real browser.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

CSS_SUFFIXES = {".css", ".scss", ".sass"}
SCRIPT_SUFFIXES = {".js", ".mjs", ".ts"}
SCANNED = {".html", ".htm"} | CSS_SUFFIXES | SCRIPT_SUFFIXES
GENERATED_DIRS = {"tokens", "dist", "vendor", "node_modules"}
failures: list[str] = []
fixed_reported: set[tuple[Path, int]] = set()

# Unicode 17.0: Extended_Pictographic outside the BMP, plus regional indicators (flag pairs) and
# skin-tone modifiers.
EMOJI_SUPPLEMENTARY = (
    (0x1F004, 0x1F004), (0x1F02C, 0x1F02F), (0x1F094, 0x1F09F), (0x1F0AF, 0x1F0B0), (0x1F0C0, 0x1F0C0),
    (0x1F0CF, 0x1F0D0), (0x1F0F6, 0x1F0FF), (0x1F170, 0x1F171), (0x1F17E, 0x1F17F), (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A), (0x1F1AE, 0x1F1FF), (0x1F201, 0x1F20F), (0x1F21A, 0x1F21A), (0x1F22F, 0x1F22F),
    (0x1F232, 0x1F23A), (0x1F23C, 0x1F23F), (0x1F249, 0x1F25F), (0x1F266, 0x1F321), (0x1F324, 0x1F393),
    (0x1F396, 0x1F397), (0x1F399, 0x1F39B), (0x1F39E, 0x1F3F0), (0x1F3F3, 0x1F3F5), (0x1F3F7, 0x1F3FA),
    (0x1F3FB, 0x1F3FF), (0x1F400, 0x1F4FD), (0x1F4FF, 0x1F53D), (0x1F549, 0x1F54E), (0x1F550, 0x1F567),
    (0x1F56F, 0x1F570), (0x1F573, 0x1F57A), (0x1F587, 0x1F587), (0x1F58A, 0x1F58D), (0x1F590, 0x1F590),
    (0x1F595, 0x1F596), (0x1F5A4, 0x1F5A5), (0x1F5A8, 0x1F5A8), (0x1F5B1, 0x1F5B2), (0x1F5BC, 0x1F5BC),
    (0x1F5C2, 0x1F5C4), (0x1F5D1, 0x1F5D3), (0x1F5DC, 0x1F5DE), (0x1F5E1, 0x1F5E1), (0x1F5E3, 0x1F5E3),
    (0x1F5E8, 0x1F5E8), (0x1F5EF, 0x1F5EF), (0x1F5F3, 0x1F5F3), (0x1F5FA, 0x1F64F), (0x1F680, 0x1F6C5),
    (0x1F6CB, 0x1F6D2), (0x1F6D5, 0x1F6E5), (0x1F6E9, 0x1F6E9), (0x1F6EB, 0x1F6F0), (0x1F6F3, 0x1F6FF),
    (0x1F7DA, 0x1F7FF), (0x1F80C, 0x1F80F), (0x1F848, 0x1F84F), (0x1F85A, 0x1F85F), (0x1F888, 0x1F88F),
    (0x1F8AE, 0x1F8AF), (0x1F8BC, 0x1F8BF), (0x1F8C2, 0x1F8CF), (0x1F8D9, 0x1F8FF), (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945), (0x1F947, 0x1F9FF), (0x1FA58, 0x1FA5F), (0x1FA6E, 0x1FAFF), (0x1FC00, 0x1FFFD),
)
# Unicode 17.0: BMP Extended_Pictographic; these render as emoji when followed by U+FE0F.
EMOJI_BMP_PICTOGRAPHIC = (
    (0x00A9, 0x00A9), (0x00AE, 0x00AE), (0x203C, 0x203C), (0x2049, 0x2049), (0x2122, 0x2122),
    (0x2139, 0x2139), (0x2194, 0x2199), (0x21A9, 0x21AA), (0x231A, 0x231B), (0x2328, 0x2328),
    (0x23CF, 0x23CF), (0x23E9, 0x23F3), (0x23F8, 0x23FA), (0x24C2, 0x24C2), (0x25AA, 0x25AB),
    (0x25B6, 0x25B6), (0x25C0, 0x25C0), (0x25FB, 0x25FE), (0x2600, 0x2604), (0x260E, 0x260E),
    (0x2611, 0x2611), (0x2614, 0x2615), (0x2618, 0x2618), (0x261D, 0x261D), (0x2620, 0x2620),
    (0x2622, 0x2623), (0x2626, 0x2626), (0x262A, 0x262A), (0x262E, 0x262F), (0x2638, 0x263A),
    (0x2640, 0x2640), (0x2642, 0x2642), (0x2648, 0x2653), (0x265F, 0x2660), (0x2663, 0x2663),
    (0x2665, 0x2666), (0x2668, 0x2668), (0x267B, 0x267B), (0x267E, 0x267F), (0x2692, 0x2697),
    (0x2699, 0x2699), (0x269B, 0x269C), (0x26A0, 0x26A1), (0x26A7, 0x26A7), (0x26AA, 0x26AB),
    (0x26B0, 0x26B1), (0x26BD, 0x26BE), (0x26C4, 0x26C5), (0x26C8, 0x26C8), (0x26CE, 0x26CF),
    (0x26D1, 0x26D1), (0x26D3, 0x26D4), (0x26E9, 0x26EA), (0x26F0, 0x26F5), (0x26F7, 0x26FA),
    (0x26FD, 0x26FD), (0x2702, 0x2702), (0x2705, 0x2705), (0x2708, 0x270D), (0x270F, 0x270F),
    (0x2712, 0x2712), (0x2714, 0x2714), (0x2716, 0x2716), (0x271D, 0x271D), (0x2721, 0x2721),
    (0x2728, 0x2728), (0x2733, 0x2734), (0x2744, 0x2744), (0x2747, 0x2747), (0x274C, 0x274C),
    (0x274E, 0x274E), (0x2753, 0x2755), (0x2757, 0x2757), (0x2763, 0x2764), (0x2795, 0x2797),
    (0x27A1, 0x27A1), (0x27B0, 0x27B0), (0x27BF, 0x27BF), (0x2934, 0x2935), (0x2B05, 0x2B07),
    (0x2B1B, 0x2B1C), (0x2B50, 0x2B50), (0x2B55, 0x2B55), (0x3030, 0x3030), (0x303D, 0x303D),
    (0x3297, 0x3297), (0x3299, 0x3299),
)
# Unicode 17.0: BMP characters that are both Extended_Pictographic and Emoji_Presentation.
EMOJI_BMP_PRESENTATION = (
    (0x231A, 0x231B), (0x23E9, 0x23EC), (0x23F0, 0x23F0), (0x23F3, 0x23F3), (0x25FD, 0x25FE),
    (0x2614, 0x2615), (0x2648, 0x2653), (0x267F, 0x267F), (0x2693, 0x2693), (0x26A1, 0x26A1),
    (0x26AA, 0x26AB), (0x26BD, 0x26BE), (0x26C4, 0x26C5), (0x26CE, 0x26CE), (0x26D4, 0x26D4),
    (0x26EA, 0x26EA), (0x26F2, 0x26F3), (0x26F5, 0x26F5), (0x26FA, 0x26FA), (0x26FD, 0x26FD),
    (0x2705, 0x2705), (0x270A, 0x270B), (0x2728, 0x2728), (0x274C, 0x274C), (0x274E, 0x274E),
    (0x2753, 0x2755), (0x2757, 0x2757), (0x2795, 0x2797), (0x27B0, 0x27B0), (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C), (0x2B50, 0x2B50), (0x2B55, 0x2B55),
)


def fail(path: Path, line: int, rule: str, msg: str) -> None:
    failures.append(f"FAIL {path}:{line} [{rule}] {msg}")


class Page(HTMLParser):
    def __init__(self, path: Path) -> None:
        super().__init__(convert_charrefs=True)
        self.path = path
        self.styles: list[tuple[int, str]] = []
        self.style_attrs: list[tuple[int, str]] = []
        self.texts: list[tuple[int, str]] = []
        self._in_style = False
        self._in_script = False
        self._style_line = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        a = {k.lower(): (v or "") for k, v in attrs}
        line = self.getpos()[0]
        if tag == "a" and "href" in a and a["href"].strip() in ("", "#"):
            fail(self.path, line, "placeholder-link", f'<a href="{a["href"]}">')
        if tag == "meta" and a.get("name", "").lower() == "viewport":
            content = a.get("content", "").lower().replace(" ", "")
            m = re.search(r"maximum-scale=([\d.]+)", content)
            if re.search(r"user-scalable=(no|0)\b", content) or (m and float(m.group(1)) < 2):
                fail(self.path, line, "zoom-lock", f'viewport "{a.get("content")}"')
        tabindex = a.get("tabindex", "").strip()
        if re.fullmatch(r"\d+", tabindex) and int(tabindex) > 0:
            fail(self.path, line, "tab-order", f"<{tag} tabindex={tabindex}>")
        if tag == "img" and "alt" not in a:
            fail(self.path, line, "img-alt", f'<img src="{a.get("src", "")}"> has no alt')
        if "style" in a:
            self.style_attrs.append((line, "{" + a["style"] + "}"))
        for name, value in a.items():
            if name not in ("style", "href", "src", "srcset"):
                self.texts.append((line, value))
        if tag == "style":
            self._in_style, self._style_line = True, line
        if tag == "script":
            self._in_script = True

    def handle_endtag(self, tag: str) -> None:
        if tag == "style":
            self._in_style = False
        if tag == "script":
            self._in_script = False

    def handle_data(self, data: str) -> None:
        if self._in_style:
            self.styles.append((self._style_line, data))
        elif not self._in_script:
            self.texts.append((self.getpos()[0], data))


def blank(m: re.Match[str]) -> str:
    return "\n" * m.group(0).count("\n")


def strip_comments(css: str, sass: bool = False) -> str:
    css = re.sub(r"/\*.*?\*/", blank, css, flags=re.S)
    if sass:  # Sass line comments; a // inside url() or after a scheme colon is not a comment
        css = re.sub(r"(?<![:(\"'/])//[^\n]*", "", css)
    return css


DECLARATION = re.compile(r"(?<![\w$-])(\$?-{0,2}[a-zA-Z_][\w-]*)\s*:\s*([^;{}]*?)(?=[;}]|\Z)")
SASS_DECLARATION = re.compile(r"(?<![\w$-])(\$?-{0,2}[a-zA-Z_][\w-]*)\s*:\s*([^;{}\n]*?)(?=[;}\n]|\Z)")
HEX_COLOUR = re.compile(r"(?<![\w&-])#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})(?![\w-])")
COLOUR_FUNCTION = re.compile(r"(?<![\w-])(rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(")
MOTION_PROPERTY = re.compile(r"(-webkit-|-moz-)?(transition|animation)(-duration|-delay)?")
DURATION = re.compile(r"(?<![\w.#-])(\d*\.?\d+)(ms|s)(?![\w-])")
UNIT_CONVERSION = re.compile(r"\*\s*1(ms|s)(?![\w-])|(?<![\w.#-])1(ms|s)\s*\*")
SIZE_PROPERTY = re.compile(
    r"(?:min-|max-)?(?:width|height|inline-size|block-size)|margin(?:-[a-z-]+)?|padding(?:-[a-z-]+)?|"
    r"gap|row-gap|column-gap|inset(?:-[a-z-]+)?|top|right|bottom|left|font-size|line-height|flex-basis|"
    r"border(?:-[a-z]+)*-radius|outline-offset|grid-template-(?:columns|rows)|grid-auto-(?:columns|rows)")
LENGTH = re.compile(r"(?<![\w.#$-])(\d*\.?\d+)(px|rem|em|ch|ex|lh|rlh|pt|pc|in|cm|mm|Q)(?![\w-])")


def inner(value: str, start: int) -> str:
    """The balanced parenthesised text that opens at value[start] == "("."""
    depth = 0
    for i in range(start, len(value)):
        depth += {"(": 1, ")": -1}.get(value[i], 0)
        if depth == 0:
            return value[start + 1:i]
    return value[start + 1:]


def check_literals(path: Path, css: str, offset: int, sass: bool = False) -> None:
    """Flag literal colours anywhere in a declaration and literal durations in motion declarations."""
    for m in (SASS_DECLARATION if sass else DECLARATION).finditer(css):
        prop, raw = m.group(1).lower(), m.group(2)
        value = re.sub(r"\"[^\"]*\"|'[^']*'", '""', raw)
        value = re.sub(r"url\([^)]*\)", "url()", value)
        line = offset + css.count("\n", 0, m.start(2))
        for c in HEX_COLOUR.finditer(value):
            fail(path, line, "literal-colour", f"{prop}: {c.group(0)}")
        for c in COLOUR_FUNCTION.finditer(value):
            args = inner(value, c.end() - 1)
            if "var(" not in args and not args.lstrip().startswith("from "):
                fail(path, line, "literal-colour", f"{prop}: {c.group(1)}({args.strip()})")
        if MOTION_PROPERTY.fullmatch(prop):
            converted = UNIT_CONVERSION.sub("", value)
            for d in DURATION.finditer(converted):
                if float(d.group(1)) != 0:
                    fail(path, line, "literal-duration", f"{prop}: {d.group(0)} (read a motion token)")
        opens = css.rfind("\n", 0, m.start()) if sass else max(css.rfind(c, 0, m.start()) for c in "{};")
        at_rule = "@" in css[opens + 1:m.start()]  # a media or container query prelude, not a declaration
        if SIZE_PROPERTY.fullmatch(prop) and not at_rule and (path, line) not in fixed_reported:
            for d in LENGTH.finditer(value):
                if float(d.group(1)) != 0:
                    fail(path, line, "literal-length", f"{prop}: {d.group(0)} (read a layout, space or type token)")


def emoji_at(text: str, i: int) -> bool:
    cp = ord(text[i])
    if cp >= 0x10000:
        return any(lo <= cp <= hi for lo, hi in EMOJI_SUPPLEMENTARY)
    if any(lo <= cp <= hi for lo, hi in EMOJI_BMP_PRESENTATION):
        return True
    followed = i + 1 < len(text) and text[i + 1] == "\ufe0f"
    return followed and any(lo <= cp <= hi for lo, hi in EMOJI_BMP_PICTOGRAPHIC)


def check_emoji(path: Path, texts: list[tuple[int, str]]) -> None:
    for line, text in texts:
        previous = -2
        for i in range(len(text)):
            if not emoji_at(text, i):
                continue
            joined = previous >= 0 and all(ch in "\u200d\ufe0f" for ch in text[previous + 1:i])
            previous = i
            if joined:  # one report per sequence (flags, ZWJ sequences, modifiers)
                continue
            at = line + text.count("\n", 0, i)
            fail(path, at, "emoji-icon", f"U+{ord(text[i]):04X} in template markup (use an icon from the theme)")


def generated(path: Path, root: Path) -> bool:
    try:
        rel = path.resolve().relative_to(root.resolve())
    except ValueError:
        rel = path
    return any(part.lower() in GENERATED_DIRS for part in rel.parts[:-1])


def check_fixed_widths(path: Path, css: str, offset: int, limit: int) -> None:
    """Flag width/min-width > limit px unless an enclosing block is a min-width media query."""
    stack: list[str] = []
    prelude_start = 0
    for i, ch in enumerate(css):
        if ch == "{":
            stack.append(css[prelude_start:i])
            prelude_start = i + 1
        elif ch == "}":
            body = css[prelude_start:i]
            guarded = any(re.search(r"@(media|container)", p) and re.search(r"min-width|width\s*>", p) for p in stack)
            if not guarded:
                for m in re.finditer(r"(?<![\w-])(min-width|width)\s*:\s*(\d+(?:\.\d+)?)px", body):
                    if float(m.group(2)) > limit:
                        line = offset + css.count("\n", 0, prelude_start + m.start())
                        fail(path, line, "fixed-width", f"{m.group(1)}: {m.group(2)}px")
                        fixed_reported.add((path, line))
            if stack:
                stack.pop()
            prelude_start = i + 1
        elif ch == ";" and not stack:
            prelude_start = i + 1


TIMING_KEY = re.compile(r"(?<![\w$.])(duration|delay|endDelay|loopDelay|visualDuration|bounce|stiffness|damping|mass)"
                        r"\s*:\s*(-?\d*\.?\d+)(?![\w.])")
STAGGER = re.compile(r"(?<![\w$])stagger\s*\(\s*(-?\d*\.?\d+)(?![\w.])")
EASE_LITERAL = re.compile(r"(?<![\w$.])(ease|easing)\s*:\s*(\[\s*-?\d[\d.,\s-]*\]|([\"'`])[^\"'`]*\3)")
STYLE_TIMING = re.compile(r"\.style\.(transition|animation)\w*\s*=\s*([\"'`])([^\"'`]*)\2")


def check_script(path: Path, text: str) -> None:
    """Literal timing in island and module code: motion options, stagger(), easing and inline style strings."""
    code = re.sub(r"/\*.*?\*/", blank, text, flags=re.S)
    code = re.sub(r"(?<![:\\\"'`/])//[^\n]*", "", code)  # line comments; a // after a scheme colon is a URL
    for rule, pattern in (("key", TIMING_KEY), ("stagger", STAGGER)):
        for m in pattern.finditer(code):
            number = m.group(m.lastindex)
            if float(number) != 0:
                what = f"{m.group(1)}: {number}" if rule == "key" else f"stagger({number})"
                fail(path, code.count("\n", 0, m.start()) + 1, "literal-motion", f"{what} (read a motion token)")
    for m in EASE_LITERAL.finditer(code):
        fail(path, code.count("\n", 0, m.start()) + 1, "literal-motion",
             f"{m.group(1)}: {m.group(2)} (read an easing token or its emitted string)")
    for m in STYLE_TIMING.finditer(code):
        if any(float(d.group(1)) != 0 for d in DURATION.finditer(UNIT_CONVERSION.sub("", m.group(3)))):
            fail(path, code.count("\n", 0, m.start()) + 1, "literal-motion",
                 f"style.{m.group(1)} = {m.group(2)}{m.group(3)}{m.group(2)} (read a motion token)")


def main(argv: list[str]) -> int:
    if "-h" in argv or "--help" in argv:
        print(__doc__)
        return 0
    if not argv:
        print(__doc__)
        return 2
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="write a machine-readable result")
    parser.add_argument("--token-roles", action="store_true", help="check literal color, size, duration and motion values")
    parser.add_argument("--template-emoji", action="store_true", help="check a project's template emoji policy")
    parser.add_argument("--fixed-width-limit", type=int, metavar="CSS_PX", help="flag widths above this limit outside a minimum-width query")
    parser.add_argument("--authored-source", action="append", default=[], type=Path)
    parser.add_argument("paths", nargs="+")
    args = parser.parse_args(argv)
    failures.clear()
    fixed_reported.clear()
    if args.fixed_width_limit is not None and args.fixed_width_limit <= 0:
        parser.error("--fixed-width-limit must be positive")
    if args.authored_source and not args.token_roles:
        parser.error("--authored-source applies when using --token-roles")
    authored = {path.resolve() for path in args.authored_source}
    files: list[tuple[Path, Path]] = []
    for arg in args.paths:
        p = Path(arg)
        if p.is_dir():
            files += sorted((f, p) for f in p.rglob("*") if f.is_file() and f.suffix.lower() in SCANNED)
        elif p.is_file():
            files.append((p, Path.cwd()))
    if not files:
        if args.json:
            print(json.dumps({"status": "skipped", "files": 0, "findings": [], "excluded": []}))
        else:
            print("SKIPPED: no supported files found")
        return 2

    scanned = {path.resolve() for path, _ in files}
    for path in sorted(authored):
        if path not in scanned or path.suffix.lower() not in SCRIPT_SUFFIXES | CSS_SUFFIXES:
            parser.error("--authored-source must name an exact scanned scene script or stylesheet")
        if not args.json:
            print(f"TOKEN ROLE CHECKS EXCLUDED: {path} (declared scene-only file; verify role and temporal evidence)")
    all_css: list[str] = []
    for f, root in files:
        text = f.read_text(errors="ignore")
        suffix = f.suffix.lower()
        source = not generated(f, root) and f.resolve() not in authored
        sass = suffix in {".scss", ".sass"}
        attrs: list[tuple[int, str]] = []
        if suffix in SCRIPT_SUFFIXES:
            if source and args.token_roles:
                check_script(f, text)
            continue
        if suffix in CSS_SUFFIXES:
            blocks = [(1, text)]
        else:
            template = "templates" in (part.lower() for part in f.parts[:-1]) or bool(re.search(r"\{[{%]", text))
            if template:  # Tera comments are not markup
                text = re.sub(r"\{#.*?#\}", blank, text, flags=re.S)
            page = Page(f)
            page.feed(text)
            blocks, attrs = page.styles, page.style_attrs
            if template and source and args.template_emoji:
                check_emoji(f, page.texts)
        for line, css in blocks:
            css = strip_comments(css, sass)
            all_css.append(css)
            if args.fixed_width_limit is not None:
                check_fixed_widths(f, css, line, args.fixed_width_limit)
            if source and args.token_roles:
                check_literals(f, css, line, sass=suffix == ".sass")
        if source and args.token_roles:
            for line, css in attrs:
                check_literals(f, css, line)

    css = "\n".join(all_css)
    moves = re.search(r"(?<![\w-])(animation|transition)(-[a-z]+)?\s*:\s*(?!none\b)|scroll-behavior\s*:\s*smooth", css)
    if moves and "prefers-reduced-motion" not in css:
        failures.append("FAIL site [reduced-motion] motion declared but no prefers-reduced-motion rule")
    if args.token_roles and re.search(r"data-mode\s*=\s*['\"]?dark", css) and not re.search(r"prefers-color-scheme\s*:\s*dark", css):
        failures.append("FAIL site [system-dark] explicit dark mode (data-mode) but no prefers-color-scheme: dark rule")
    if re.search(r"outline\s*:\s*(none|0)\b", css) and ":focus-visible" not in css:
        failures.append("FAIL site [focus-removed] outline removed and no :focus-visible rule")

    if args.json:
        print(json.dumps({"status": "failed" if failures else "passed", "files": len(files),
                          "findings": failures, "excluded": sorted(map(str, authored))}))
    else:
        for line in failures:
            print(line)
        print(f"{len(files)} file(s) scanned, {len(failures)} failure(s)")
        print("NOT CHECKED: rendering, overflow, contrast, target size, focus visibility, behaviour")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
