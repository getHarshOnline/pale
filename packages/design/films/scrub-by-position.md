+++
description = "Use it to edit a sequence the reader drives by scrolling, so each position reached from above or below shows the same settled picture."
uses = [{ scope = "local", resource_id = "films" }]
see_also = [{ scope = "local", resource_id = "films/choose-the-clock" }]

[guide]
exposure = "internal"
+++

# Edit a reader-driven sequence

A scrubbed sequence shows the same settled picture at a position reached from above or below. A short catch-up may approach that target after input stops. It cannot travel past it. Compare the approach and the settled frame separately. Once settled, the image is a composed still and drawing can sleep. Ambient playback and a linear film have [Choose a sequence clock and placement](choose-the-clock.md).

Write a pacing table as data for the sequence. Use the page's scroll distance rather than a changing mobile viewport height. Start with scroll proportional to travel. If one passage needs different density, give that row its own reason. Put words on held frames. Bring the words into the reading area during the hold so distance does useful work. Between scenes, give the eye a resting frame. Compose the final frame to lead to the next step.

The picture can pin while the words scroll as ordinary content. Keep wheel, trackpad, touch, keyboard, scrollbar, find and fragment links in charge of page scrolling. Give each beat an address and open that address at its resting frame. Include a link past the sequence. Keep focus and reading order in the DOM sections. When a pointer moves a local reveal mask, keep the frame, controls and text stationary. Offer its meaning in the stills or words, or through equivalent focus and touch paths. Use the composed still on coarse pointers without hover.

## Design holds, chapters and a way out

Keep the reader's scroll and input. Separate view progress from authored timeline progress. Write down each beat's travel and holds so the observer can reproduce the important positions. The same position should resolve to the same subject, labels and state from either direction. A still reader deserves a settled frame that states the idea rather than a transition frozen halfway.

Make an addressable sequence start at the chapter or still that belongs to the route. Keep a way to skip the region, return through Back, and reach its text and links without traversing every frame. Test with slow reverse, fast reverse and entry from a deep link, as well as the normal direction. Separately verify that a linear film can play through its own controls without acquiring the page's scroll clock.

## Worked example

| Beat | Span | Hold in | Hold out | Frames | Words |
| --- | --- | --- | --- | --- | --- |
| See the starting arrangement | 240 px | 100 px | 140 px | 1–24 | Starting arrangement |
| Remove the constraint | 480 px | 100 px | 200 px | 25–72 | What moves and why |
| Compare the result | 240 px | 120 px | 180 px | 73–96 | Difference and next step |

These distances are for this example. Replay a settled midpoint by scrolling both ways. Follow the second beat's link with the browser toolbar open and again when it retracts. Then use the skip link and keyboard find. If a hold shows no useful change while the reader scrolls, bring its words into view or shorten that span.
