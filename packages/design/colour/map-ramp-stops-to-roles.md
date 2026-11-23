+++
description = "Use it to give each ramp stop a job on a real surface and to derive control states once for the controls that share a rule."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "colour/css" }]

[guide]
exposure = "internal"
+++

# Assign ramp stops and derive control states

Give each stop a job. Derive repeated state relationships once when the same rule should govern several controls.

## Assign roles on a real surface

Put reading ink, muted text, separators, focus, emphasis and operational signals on the surfaces where they occur. A warm print-like ground can use a tinted stock, low-chroma pigment and a small spot color. A film may need saturated fields or emitted light. Start with the content, light, geometry and material of the work, then choose the palette. If warm stock and an orange accent keep returning regardless of the subject, compare the whole treatments with accent suppressed.

## Derive repeated states from the same job

For a system that dims or darkens a ground on hover, use a perceptual mix toward the reading ink. In light mode that makes the ground darker. In dark mode it makes the ground lighter. Choose the fractions for each mode. The same numeric step can look smaller on a dark ground. Store the rule in the token system when several controls share it. Mark a held choice with the state cue the interface uses, and keep accent, focus, selection and signals distinguishable. Check the derived background against text and against neighboring states on the page.

## Worked example: pressed and held

| State | Ground relationship | What stays visible |
| --- | --- | --- |
| Resting | Panel role | Label and control edge. |
| Pressed | The panel role mixed toward ink with its pressed fraction | Label, focus and a distinct press. |
| Held | The held-state ground and the control’s persistent marker | Which choice is still active after release. |

If the two derived grounds collapse in dark mode, tune the mode’s fractions or change the persistent marker. Keep the shared rule where the consumer can find it. [Apply color roles in CSS](css.md).

## Sources

- [Radix Colors: understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale): a first-party example of assigning scale steps by job (grounds, component states, borders, solids, text) with light and dark scales paired.
- [Carbon Design System: color overview](https://carbondesignsystem.com/elements/color/overview/): a first-party example of a role layer over a palette and themes built from the same roles.
