+++
description = "When the timed artwork is the asset and the destination can deliver the format, player and dependencies."
uses = [{ scope = "local", resource_id = "figures" }]
official_sources = [{ label = "Deliver an authored animation with a controlled player", url = "https://github.com/airbnb/lottie-web" }]

[guide]
exposure = "internal"

[stack]
+++

# Deliver an authored animation with a controlled player

Use an authored player when the timed artwork is the asset and the destination can deliver the format, player and its dependencies. If the reader needs an accessible timeline, real controls or an interactive model, choose the interface separately. A rendered film may be the smaller delivery for a fixed passage. Check the player’s supported features, any embedded media, and the scene’s original timing and color.

Choose the player from the authored format and required interaction. A Lottie export can suit a timed vector passage; a Rive asset can carry authored states and transitions when its runtime and accessibility route fit. Test the actual exported features, fonts, images and runtime version. Neither player automatically adopts the page’s colors, tokens or motion preferences. A fixed passage can instead ship as video if the reader needs playback rather than live state. Keep meaningful controls and descriptions in the surrounding accessible interface.

Keep the selected source, editable master, rights and derivative relationship with the asset. A downloaded file needs an assigned role; an approved source is not rejected merely because it was downloaded. For a control state or simple entrance already handled by CSS, compare the benefit against the added player. Illustrated movement or an authored state machine may justify that cost.

Separate interface roles from depicted action. A status color or button response follows the surrounding system's roles; a character's movement, scene illumination or an authored cut follows the selected composition. Adapt actual interface roles through runtime properties or authored variants and check all required appearances. Recoloring every light or forcing every action onto one interface easing curve can destroy the selected work.

Vet and share the player code, then create only the instances the composition needs. Measure the delivered player, artwork, decoders and supporting media together. Verify that the target can transport the chosen files and load its module or WASM dependencies; report unsupported delivery at that interface's owner. A film uses its selected native runtime's animation support and seeking contract. Verify an integration or render a derivative when that runtime cannot directly use the player. A page's transport limitation does not settle the film's production options.

## Worked example: mount a Lottie passage with a still

```js
const animation = lottie.loadAnimation({
  container: mount,
  renderer: "svg",
  autoplay: false,
  loop: false,
  animationData,
});
animation.addEventListener("DOMLoaded", () => {
  if (shouldPlay()) animation.play();
});
```

Ship the artwork through the project’s asset route. Keep a composed still until the player is ready and for the unsupported or reduced form. Check that the same meaning and surrounding text survive both. A paused player should resume at the intended position. When the component leaves, remove observers and listeners and destroy the instance. Compare the decoded or drawn result to the reference, including masks, gradients, repeat boundaries and actual legibility. A small animation file can still create an expensive tree.

This example shows the start decision, not a complete lifecycle. `shouldPlay()` belongs to the host's current visibility, pause and motion policy. Wire later changes too: a preference change or a hidden document must not leave a player running under stale startup conditions. Retain the still on load failure and verify the first complete rendered frame before replacing it; a DOM-ready event alone does not prove that every image or font is usable. For film capture, verify repeated cold seeks and seek-back with assets ready instead of advancing a page loop.

Hide purely decorative artwork from assistive technology. Give informative artwork a meaningful text alternative and retain facts and controls in the accessible interface. A self-playing loop follows the destination's pause requirements and the chosen time model. Check reduced motion, no script and failed-player delivery separately; a host fallback is not a redesign of the authored film.

Measure first-frame cost, renderer nodes or drawing work, ongoing activity and memory at the busiest state. Suspend offscreen and hidden work, and inspect repeated mount and teardown for retained instances. Record the source version, renderer, tested export features and observed limits with the delivery. Use [the drawing-layer budget](budget-a-scene.md) when comparing a player with video, frames or another live renderer.

## Sources

- [lottie-web](https://github.com/airbnb/lottie-web): exported data, supported renderers, playback controls, readiness events and instance destruction for the version being used.
- [Rive Web runtime](https://rive.app/docs/runtimes/web/web-js): renderer-dependent feature support, state-machine integration and the JavaScript/WASM delivery path.
- [WCAG Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html): when automatically moving page content needs a way to pause, stop or hide it.
- [WAI decorative images tutorial](https://www.w3.org/WAI/tutorials/images/decorative/): keeping decorative media out of the accessibility reading path while informative media retains its alternative.
