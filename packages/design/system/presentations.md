+++
description = "Use it to design the light, dark and reduced renditions as usable states and to check each route the project offers to reach them."
uses = [{ scope = "local", resource_id = "system" }]
see_also = [{ scope = "local", resource_id = "responsive" }, { scope = "local", resource_id = "responsive/capabilities" }]

[guide]
exposure = "internal"
+++

# Design each presentation and check how it is reached

Color mode and motion preference can combine as light, light reduced, dark and dark reduced. Compose the affected states as usable renditions. Derive dark ground, ink, key and depth for a dim setting. When travel is reduced, use a settled change in material, light or composition to carry the same meaning. An identity that is dark at rest may still have separate light and dark presentation choices.

Test the routes the project offers. The system setting and an explicit page choice should show the same rendition when they resolve to the same state. A missing saved choice follows the project's declared default. It may differ from the system color mode. Test each separately. A dark screenshot cannot show which path took effect. Compare computed ground, text, accent, surface and focus values for each pair. Include one live region and an open dialog when the site uses them. A rule that only takes effect under an explicit attribute can leave the system route different. Measure text and focus contrast in both color modes.

Check that explicit light can override a dark system and explicit dark can override a light system. Navigate between two real pages and inspect first paint, including any brief flash. Block storage and check that the choice still works for the current view. Where the site cannot remember the choice, say so beside the setting. Check the browser chrome color on a target device where the site supplies it with `theme-color`. Change a preference during the affected interaction. The result should settle, the state remain readable and focus stay useful.

The input context depends on space, pointer, hover and the reader's task. A small window can still have a mouse and a keyboard. A large touchscreen can lack hover. Preserve shared facts, sources, status, units, destinations and paths. A margin note or hover preview can serve a desk if the destination remains reachable in hand. A compact next and previous control can serve a phone without adding a fact that other readers cannot reach. Prefer one markup order and place the display around it. See [Recompose an interface without losing its task](../responsive.md).

A capability can fail in any of these states. Missing scripts, blocked fonts, failed graphics, reduced transparency, higher contrast and forced colors need [Keep the task when a capability is absent](../responsive/capabilities.md). Test the combination that matters for the task. Keep role values in the project's token source, and keep the content and local rendition choices in the project. Use browser observations for the resulting color, focus and placement.

## Worked example

A study's wide treatment shows its details in a margin. The narrow treatment keeps the same figure and data link, and puts the details in flow. The project has a paper default, so a fresh browser with a dark system setting still opens light. Choosing System opens the same dark rendition as choosing Dark. Compare the two dark views, then switch to reduced motion while the disclosure opens. The result and focus should remain clear. Disable the enhancement and confirm the note and source remain reachable.
