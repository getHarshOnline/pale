+++
description = "Choose or repair color roles, perceptual relationships, gamut, states and light and dark palettes for the intended design."
uses = [{ scope = "local", resource_id = "access/web" }]
see_also = [{ scope = "local", resource_id = "colour/choose-a-space" }, { scope = "local", resource_id = "colour/compare-perception" }, { scope = "local", resource_id = "colour/map-gamut" }, { scope = "local", resource_id = "colour/contrast-wcag" }, { scope = "local", resource_id = "colour/contrast-apca" }, { scope = "local", resource_id = "colour/build-a-ramp" }, { scope = "local", resource_id = "colour/map-ramp-stops-to-roles" }, { scope = "local", resource_id = "colour/design-a-dark-palette" }, { scope = "local", resource_id = "colour/meaning-and-context" }, { scope = "local", resource_id = "colour/compare-by-role" }, { scope = "local", resource_id = "colour/token-roles" }, { scope = "local", resource_id = "colour/check-the-rendered-palette" }, { scope = "local", resource_id = "colour/css" }]

[guide]
+++

# Choose color by role, perception and delivery

Use this page to build an interface palette, change a role or diagnose a color problem in the delivered work. Start with the direction, material, real text and controls. Decide what each color does and which appearances the interface must support. An authored film or photograph follows its own color decisions.

Give foreground, ground, boundaries, focus and operational states separate roles where they can change independently. Start with a lightness ladder. Check its shape in greyscale, then use hue and chroma to bring out the material, emphasis and distinctions the page needs. Compare the result with the reference at reading scale.

| If you are doing this | Read this |
| --- | --- |
| Choose between OKLCH, OKLab, CIELAB, HCT and linear light | [Choose a color space for the operation](colour/choose-a-space.md) |
| Explain why brightness, lightness and luminance disagree | [Tell lightness, brightness and chroma apart](colour/compare-perception.md) |
| Convert P3 or OKLCH into a smaller gamut | [Map a color to its destination gamut](colour/map-gamut.md) |
| Build the ladder and assign stops to controls | [Build a perceptual ramp before assigning roles](colour/build-a-ramp.md) and [Assign ramp stops and derive control states](colour/map-ramp-stops-to-roles.md) |
| Derive and test paired modes | [Design the same roles for a dark room](colour/design-a-dark-palette.md) |
| Compare palettes without relying on hue rules | [Compare color relationships on the page](colour/compare-by-role.md) |
| Distinguish a link, state, category or series | [Make color one cue for a meaning](colour/meaning-and-context.md) |
| Check a text or focus pair on a rendered surface | [Measure contrast on the surface that people read](colour/contrast-wcag.md) and [Use APCA to diagnose a hard-to-read pair](colour/contrast-apca.md) |
| Store shared roles or write CSS | [Store color tokens by job](colour/token-roles.md) and [Apply color roles in CSS](colour/css.md) |
| Find out why a palette fails in use | [Diagnose a palette in the running interface](colour/check-the-rendered-palette.md) |

Color references adapt [meodai’s color skill](https://github.com/meodai/skill.color-expert/tree/28e49f7457d7aa92010688e69975c16d2ffe4b01/). The attribution and CC BY 4.0 terms accompany the color references.

## Before handing off

- [ ] Does each role serve its actual surface and state?
- [ ] Do the intended gamut, built values and rendered colors agree?
- [ ] Do text and controls remain legible, with state and meaning carried beyond hue alone?
- [ ] Where both modes are offered, do they work on the same real content?
