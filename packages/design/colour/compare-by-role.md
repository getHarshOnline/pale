+++
description = "When a harmony formula, emotion label or swatch sheet is choosing the palette before you have seen it in use."
uses = [{ scope = "local", resource_id = "colour" }]

[guide]
exposure = "internal"
+++

# Compare color relationships on the page

Use this when a harmony formula, emotion label or swatch sheet is deciding the palette before you have seen it in use.

A color wheel can propose candidates. It does not tell you whether two roles work in context. Wheel geometry depends on the chosen model. Equal angle changes in HSL are unequal perceptual steps. Hue categories and cultural associations can guide a question, but lightness, vividness, area and adjacency can change the answer. Muted and pale sets often feel quieter across several hues. Check the actual direction rather than assuming blue makes the surface trustworthy.

Albers’s examples show one sample looking like different colors against different grounds. Compare the same swatch at reading scale, with its neighbors, text, borders and roles. Test whether the hierarchy survives with accent removed. Name the material and the visible effect you want before using a wheel or formula.

## Put the color beside what changes it

Compare candidate colors in the role, size, background and viewing context where they will ship. Adjacent colors change how a swatch reads. A color that looks quiet on a board can jump forward on a dark bed or disappear on a mixed surface. Use a neutral surround and the actual surround as two separate observations. A color wheel can suggest alternatives. It cannot establish perceived balance, contrast, meaning or legibility.

Choose hue, chroma and lightness for the purpose of the role. Move one when it lets you isolate the cause. Compare the complete treatments with accent removed, in grayscale or with a color-vision simulation as useful diagnostics. Keep labels, position or shapes to carry meaning the simulations expose. A palette can share correct values with another identity. Distinctness comes from the composition of choices, not a count of unique swatches.

## Worked example: compare two candidates for a callout

One candidate uses a hue from a complementary wheel pair. The other uses a closer hue with a lighter ground and smaller chromatic mark. In the actual article the first distracts from the title and makes its thin text hard to read. The second keeps emphasis with the title while the mark identifies the callout. Compare the pair in light, dark and greyscale, then check the text criterion. Judge the result from its role.

## Sources

- [CSS Color Module Level 4: OKLab and OKLCH](https://www.w3.org/TR/css-color-4/#ex-hsl-sucks): the worked example showing that HSL hue angles are perceptually uneven while OKLCH hue differences track visual separation.
- [Björn Ottosson: A perceptual color space for image processing](https://bottosson.github.io/posts/oklab/): the Munsell-slice comparisons showing how uneven older spaces are, and why hue relationships must be computed in a perceptual space.
- [Yale University Press: Interaction of Color (Josef Albers)](https://yalebooks.yale.edu/book/9780300179354/interaction-of-color/): the citation only: the book whose premise, that colour is relative and seen in relationship, grounds judging roles in context; the page itself does not carry the argument.
- [Upstream colour-expert skill: colour theory's dogma problem](https://github.com/meodai/skill.color-expert/blob/28e49f7457d7aa92010688e69975c16d2ffe4b01/references/contemporary/color-theory-dogma-problem.md): the approachable-versus-accurate argument and the measured counter-examples to wheel complements.
- [Upstream colour-expert skill: harmony by character](https://github.com/meodai/skill.color-expert/blob/28e49f7457d7aa92010688e69975c16d2ffe4b01/references/techniques/youre-wrong-about-color-harmony.md): the character-first account of harmony and why hue intervals ignore each hue's gamut shape.
- [Upstream colour-expert skill: Itten's seven contrasts, a critical review](https://github.com/meodai/skill.color-expert/blob/28e49f7457d7aa92010688e69975c16d2ffe4b01/references/historical/itten-seven-contrasts-critical-review.md): the measured critique of Itten's plates and claims.
