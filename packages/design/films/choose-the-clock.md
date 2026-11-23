+++
description = "Use it to pick the clock and delivery surface of a sequence, a linear film or a scene mounted in a page, and to verify seeking."
uses = [{ scope = "local", resource_id = "films" }]
see_also = [{ scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "assets" }, { scope = "local", resource_id = "editing" }]

[guide]
exposure = "internal"
+++

# Choose a sequence clock and placement

## Use the correct clock and delivery surface

A linear film owns an authored, paused and seekable timeline. Adapt useful page geometry or mechanisms to it without importing pointer history or scroll controllers. Verify cold seeks and seek-back as well as ordinary playback. For mounted scenes, resolve animation targets in the intended mount and check matching siblings in the joined render; a standalone scene cannot establish that isolation. Native runtime APIs and scene editability remain with their actual tool and applicable references.

For a reader-scrubbed page sequence, apply [Design motion that helps people use the interface](../animate.md): the same settled position should select the same picture from either direction. Keep reading, navigation, input and a way past the sequence usable. Check holds, changing viewport geometry and scrolling on the supported devices. Do not impose the scrubbed time model on ambient media or a film. Use [Compose and check a live visual region](../figures.md) for its actual page host and fallback lifecycle, rather than importing another project's page modes or fixed amplitude scale.

Choose ratio, raster, rate and essential-content clearance for the actual placement. Recompose when a crop loses the subject, relation or words; reuse a crop when those survive. Native interface proportions need not be scaled with editorial type. Fixed video pixels do not recolor themselves when their host changes theme. Use [Make an asset that belongs in its destination](../assets.md) for the selected media and [Edit the actual footage and sound](../editing.md) for cuts, clocks, color, captions and sound.

## Worked example

A 16:9 film plays on an article, and a later version follows scroll. Keep the film’s timeline and player controls for the first delivery. Give the scroll version a stable position-to-frame mapping and a readable held view. Test both directions at the same position. On mobile, choose a crop or recut that keeps the subject and words inside the intended player.
