+++
description = "Use it to assign reading, display, system and figure roles before choosing faces."
uses = [{ scope = "local", resource_id = "type" }]

[guide]
exposure = "internal"
+++

# Choose reading, display, system and figure roles

## Assign roles before choosing faces

Distinguish identity-setting display, sustained reading, short operational strings and the figure behavior needed for comparison. These are useful jobs, not a requirement to install four fonts. A family can serve several roles when the distinction is intentional and visible. Choose relationships that support the project’s identity; familiar type can be correct, but an unspecified fallback must not be presented as an authored decision.

Choose reading texture for sustained comprehension and display character for the moments that need it. Test them together on real paragraphs and difficult titles. Operational strings need unambiguous characters and reliable copying; monospace can help, but it is not a universal rule for all navigation or technical explanation. Treat an error's explanatory sentence as content rather than automatically turning it into terminal costume. Preserve any stronger selected project policy with that project's own system.

For aligned values and changing counters, request tabular lining figures with `font-variant-numeric: tabular-nums lining-nums`; prose can need proportional figures. Prefer the named `font-variant-*` properties to low-level `font-feature-settings` where they express the choice. Check the delivered font actually has the requested feature by comparing equal-length digit runs and observing a changing counter. A property or token name can request a feature the font lacks. Signs, units and qualifiers remain attached to the values they modify.

## Give each role a recognizable texture

| Role | What it carries | A sign the choice is wrong |
| --- | --- | --- |
| Display | A title, an opening or a moment that carries the subject’s character | The whole effect depends on enlarging the reading face or on the platform’s default stack. |
| Reading | Sentences people need to understand, including explanations in a tool | A long list switches faces between rows or an error paragraph becomes monospace for atmosphere. |
| System | Commands, addresses, keys, timestamps, identifiers and machine state | A reader has to parse long prose in a short-measure role, or the labels overpower the content. |
| Figures | Aligned values, dates, comparisons and counters | Columns drift or a changing counter moves neighboring text. |

The number in a large comparison can take the display scale and keep the numeric features. A role can share a family with another and use an actual axis for its inflection. Look at texture and comprehension in the same page. Check real italic and small caps if the copy relies on them. Apply capitals through the shared CSS role, for example `text-transform`, when appropriate for the language, rather than rewriting the source string. Leave the case of commands, identifiers and paths alone. Remove a label that simply repeats a heading. Keep it if it tells the reader a real kind, state, address or date.

Judge display width, weight and optical size beside the real reading face, including narrow wrapping; a specimen alone cannot show competition. For reading, check a full paragraph, real italic, similar glyphs and inline numbers. A system label can recede through position and weight while remaining readable at rest. Use that role for navigation when the labels behave like addresses or commands, and preserve the language and script’s case and spacing rules.

When the accepted brief calls for a distinctive display voice and the current treatment reads as generic, compare a stronger inflection of the selected face in context. A restriction on opening a new font search still allows work with the chosen faces; it does not authorize losing the display role. Keep the selected family's supported ranges and the system's shared role behavior. Record intentional family sharing and mono display with the project's type decisions, rather than allowing either to arise accidentally through inheritance.

Judge text color and opacity together on the actual background. A secondary color combined with a recessive opacity can fall below the system's reading floor even when each setting looks reasonable alone. Required text remains readable at rest; hover or focus cannot be its only legible state. Apply short capital labels through the shared role's case and tracking rules, preserve the authored string and inspect copying and accessible text. Use `font-variant-caps: all-small-caps` only with verified small-cap glyphs when that is the intended treatment.

## Worked example

In a report, give the title a display face and the long account a reading face. The label “Run check” and its explanatory sentence can use the reading face at different scales. Render the machine command itself in a face with unambiguous 0, O, 1, I and l. Try proportional figures in a paragraph and tabular figures in a comparison column. Copy a command from the output to check that its literal characters survive.

## Sources for the decision

- [CSS Fonts: generic families](https://www.w3.org/TR/css-fonts-4/#generic-font-families): distinguish a generic fallback category from a delivered family chosen for identity.
- [MDN: numeric variants](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-variant-numeric): choose figure shape and spacing separately, then verify the font's features.
- [MDN: feature settings](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-feature-settings) and [capital variants](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-variant-caps): prefer the higher-level controls and inspect real versus synthesized glyphs.
- [Practical Typography: body text](https://practicaltypography.com/body-text.html) and [monospaced fonts](https://practicaltypography.com/monospaced-fonts.html): compare sustained reading requirements with the jobs that benefit from fixed-width characters.
- [Carbon typography](https://carbondesignsystem.com/elements/typography/overview/): an example of related type families with distinct roles, not a mandatory face selection.
- [Practical Typography: letterspacing](https://practicaltypography.com/letterspacing.html), [all caps](https://practicaltypography.com/all-caps.html) and [small caps](https://practicaltypography.com/small-caps.html): spacing and texture of short capital runs, and the cost of counterfeit small caps.
- [MDN: text transform](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/text-transform): language-specific case mapping while retaining the authored source string.
