+++
description = "Use it to break lines by sense against the longest admitted titles and real labels, and to keep text accurate when copied."
uses = [{ scope = "local", resource_id = "type" }]

[guide]
exposure = "internal"
+++

# Keep sense and precision when lines break

## Preserve meaning through line breaks and copying

Use the longest admitted title, long identifiers, real labels and the narrowest relevant arrangement. Break display by sense. A hard return that looks good in one capture can fail in another; choose responsive composition and supported breaking controls before editing the author's claim. Truncation is appropriate only where the task still exposes the complete content through an understandable route. It must not silently remove the title or qualification the reader needs.

Preserve the source's value, precision and units. Bind number/unit and sign/value relationships using language-appropriate characters or structure that survive copying and alternate outputs. Keep a qualifier beside the correct figure. Show the actual relationship between paired values; a slash or bare juxtaposition can turn a comparison into an apparent ratio. Styling cannot repair missing semantic information.

Adjacent labels need textual separation as well as visual spacing, so copying and assistive technology do not fuse them. Keep code and literal input copyable; typographic substitutions suitable for prose can corrupt a command. Declare the document language and mark passages in another language so hyphenation, punctuation and case behavior follow the text. `hyphens: auto` also requires the browser's dictionary for that language. Use it where the reading role permits hyphenation and keep display breaks deliberate. Do not apply Latin tracking or break rules blindly to joining scripts. The [W3C account of line breaking](https://www.w3.org/International/articles/typography/linebreak.en) explains why language and script need separate treatment.

Inspect short final lines, a heading stranded from its paragraph and label/value breaks in the actual output. `text-wrap: balance` suits short display blocks; engines limit how many lines they balance, so test the longest title too. `text-wrap: pretty` can improve paragraph endings at a processing cost; do not apply it indiscriminately to long lists or tables. `widows` and `orphans` control lines at fragment boundaries such as pages and columns, not a short final line in an ordinary continuous screen paragraph. Check the selected renderer's support and retain a readable baseline where these controls are unavailable.

## Inspect the places where a line can change meaning

Bind quantities in the text or template when that relationship must survive feeds and copying: a language-appropriate no-break space where spacing belongs, or a word joiner (`U+2060`) where it does not. Check how invisible joining characters affect downstream consumers; preserve executable and machine-readable strings exactly. Keep short ranges together; when a long range must break, prefer after its range mark while leaving endpoints and qualifications unambiguous. Negative signs, approximation marks and footnote markers stay with their figures. Allow a long address or path to break at a useful boundary without changing the copyable value. Give code a local scroll route if the line itself matters.

Test unusually long titles, translated labels, long numbers and punctuation in the delivered face, at the real viewport and zoom and with the fallback face. Some scripts need more room around their glyphs even at display sizes; a Latin spacing setting is not their default. For a bad final line, adjust measure, supported wrapping or phrase placement first. A display title can move within its approved scale range; do not shrink sustained reading to make it fit. Reword only with the content owner's authority, preserving the claim; a paragraph runt alone is no reason to rewrite an author's text.

For prose punctuation, keep an opening quote, bracket or list marker with what follows and closing punctuation with what precedes it. Use the language’s quotation marks, apostrophes and range marks; preserve literal characters in commands and code. If the direction calls for hanging punctuation, apply `hanging-punctuation` as an enhancement and inspect both the supporting renderer and normal alignment without it. Justified screen text needs suitable language-aware hyphenation and an inspection for rivers at the actual widths.

## Worked example

Use “from 3.0 kg to 4.5 kg” for a change in mass. A bare “3.0 / 4.5 kg” could read as a ratio. Use the language’s appropriate spacing between each number and unit. Check wrapping in a narrow column and copy the values from the page. In a long heading, test a meaningful break without inserting a hard return that leaves the mobile first line empty. Test a joining script separately before sharing tracking or wrapping rules with Latin text.

## Sources for the decision

- [MDN: text wrapping](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/text-wrap) and [widows](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/widows): distinguish continuous wrapping from page and column fragmentation.
- [MDN: hyphenation](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/hyphens): language, dictionary and explicit break requirements.
- [Unicode line-breaking algorithm](https://www.unicode.org/reports/tr14/): joining and break opportunities, including no-break spaces and word joiners.
- [Practical Typography: nonbreaking spaces](https://practicaltypography.com/nonbreaking-spaces.html): common number, unit and reference relationships that should survive a wrap.
- [MDN: hanging punctuation](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/hanging-punctuation): available controls and current renderer support.
- [Practical Typography: hyphens and dashes](https://practicaltypography.com/hyphens-and-dashes.html) and [straight and curly quotes](https://practicaltypography.com/straight-and-curly-quotes.html): punctuation choices and literal-code exceptions.
