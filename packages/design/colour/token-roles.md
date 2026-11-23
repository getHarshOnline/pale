+++
description = "Use it to decide what each colour token means and which storage space and fallback the build actually delivers."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "colour/map-gamut" }]

[guide]
exposure = "internal"
+++

# Store color tokens by job

Choose what the token means, then choose a storage space and fallback that the build actually delivers.

## The DTCG colour value

A token of `$type` `color` has an object `$value`:

| Property | Required | Meaning |
| --- | --- | --- |
| `colorSpace` | yes | one of the module's spaces: `srgb`, `srgb-linear`, `hsl`, `hwb`, `lab`, `lch`, `oklab`, `oklch`, `display-p3`, `a98-rgb`, `prophoto-rgb`, `rec2020`, `xyz-d65`, `xyz-d50` |
| `components` | yes | the channel values for that space; any may be the keyword `none` (missing, for example the hue of a neutral) |
| `alpha` | no | 0 to 1; assumed 1 when omitted |
| `hex` | no | a fallback representation in six-digit CSS hex, six digits so it never conflicts with `alpha` |

The module leaves gamut mapping and conversion to translation tools and warns that the algorithm choice changes appearance. Treat any tool's conversion as a step to verify, not a given.

## Storage space versus design space

- Design in OKLCH; store in the space the build and consumers handle reliably. Storing sRGB components with a hex fallback is valid; the OKLCH intent is then documented in the token's description or recoverable by conversion.
- When a role is authored in OKLCH or Display P3, the stored `hex` (and every sRGB rendering) needs a deliberate gamut mapping when the authored value is out of range (see [Map a color to its destination gamut](map-gamut.md)).
- If the token check measures the sRGB value, confirm the browser emits that value for the corresponding path.

## Name the job and keep the relationships visible

A component consumes a role such as `focus` or `strong-line`. The palette can have numbered primitives underneath. Store a missing hue as `none` where the format and build support it. A named color in a design conversation can be useful. It is a poor role name when the job must survive a hue change. Alias two roles if the design intends the same decision. Equal values today do not justify coupling them.

When a new role enters a shared system, change the role contract, the values that implement it and the consumer checks together. Scene color in a film or imported art lives with the asset. If a link marks a different publication, the current publication owns how the cue is drawn and the target can supply its mark or accent. Keep theme bindings and role names with the system that builds them.

## Worked example: decouple focus from the brand accent

A `focus` role aliases `accent` in one mode. After the brand accent changes, it disappears against a button’s fill. Give focus a separate value on that surface and run the focus check in both modes. If a role stays shared by design, keep the alias and test both uses after changes.

## Sources

- [DTCG Color module 2025.10](https://www.designtokens.org/tr/2025.10/color/): the colour `$value` object (`colorSpace`, `components`, `alpha`, `hex`), the supported spaces, the `none` keyword and the non-normative note on gamut mapping.
- [DTCG Format 2025.10](https://www.designtokens.org/tr/2025.10/format/): token structure, `$type`, `$description`, aliases and groups.
- [DTCG Resolver 2025.10](https://www.designtokens.org/tr/2025.10/resolver/): how sets and modifiers resolve a world and a mode into one token tree.
- [Carbon Design System: color overview](https://carbondesignsystem.com/elements/color/overview/): a first-party example of role tokens named by job over a palette, shared across themes.
