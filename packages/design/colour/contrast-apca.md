+++
description = "Use it when a pair passes WCAG and still reads poorly, as research context for size, weight and polarity; report WCAG conformance separately."
uses = [{ scope = "local", resource_id = "colour" }]

[guide]
exposure = "internal"
+++

# Use APCA to diagnose a hard-to-read pair

Use a polarity-sensitive research measure when a pair passes WCAG and still reads poorly. Report WCAG conformance separately.

## APCA as research context

APCA (the Accessible Perceptual Contrast Algorithm, from the SAPC research) models contrast as polarity-sensitive and dependent on size and weight, reporting a lightness-contrast value rather than a ratio. It can help explain size, weight and polarity differences when a passing pair reads poorly. It is not a WCAG 2.2 conformance method and carries its own licence terms. Use it to choose **between** pairs that already pass the WCAG gate, never to pass a pair that fails it, and never report an APCA value as conformance.

## Worked example: two dark-mode labels

Two labels pass the required WCAG ratio. One is larger and heavier. The other uses a fine face at a small size and looks weak on the dark surface. Compare both at their actual sizes and weights using the same APCA implementation. Try a heavier face or a stronger lightness difference on the weaker label. Recheck the adjusted pair on the page and record the WCAG ratio. An APCA result cannot turn a failing ratio into a passing one.

## Sources

- [WCAG 2.2 Understanding: Contrast (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html): the rationale behind 4.5 : 1 and 3 : 1 (contrast sensitivity at about 20/40 vision), the sRGB assumption, no rounding, and the advice about thin or unusual fonts.
- [SAPC-APCA repository](https://github.com/Myndex/SAPC-APCA): the APCA model, its polarity and size dependence, its documentation and its licence terms; research context only.
