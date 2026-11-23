+++
description = "Use it when cutting or assembling footage, to probe the inputs' dimensions, frame rate and timebase and to keep their clocks."
uses = [{ scope = "local", resource_id = "editing" }]

[guide]
exposure = "internal"
+++

# Keep the clocks when cutting or assembling footage

## Inspect the inputs and preserve the clocks

Probe dimensions, rotation, duration, frame rate and timebase, audio streams and color interpretation. Do not infer camera role or HDR from resolution. Retain source selection and order instead of silently discarding an unusual format. Normalize only as the selected task needs. An HDR-to-SDR transform changes pixels; labeling untransformed HDR pixels as SDR is not that transform. Upscaling should remain visible as a delivery property, not claimed source detail.

When repeated native seeking needs dense keyframes, prepare them at the selected source or output scale. That need does not require every source to be upscaled or fitted to a landscape preset.

Keep source media timestamps, retained-cut offsets, local scene time and master assembly time distinct. Preserve the kept source intervals and their mappings, including room tone, shifted media and assembly trims. Align to the actual timebase. A changed cut makes downstream captions, word anchors and action timings stale until remapped or re-observed. Check difficult joins and early and late alignment, rather than assuming one initial synchronization proves the entire edit.

Transcription can propose cuts, but inspect takes against the source sound, footage and intended script. A restart is not always expendable: lists, emphasis, multilingual repetitions and alternate endings can resemble retakes. Word timestamps can omit a consonant or include a pause. Waveform or RMS thresholds can misclassify breaths and quiet speech. Listen to cut heads and tails and the assembled passage before accepting a boundary.

For interface captures, retain input-event timing separately from the first visible response. Capture sampling, trimming and frame-rate conversion can move that response relative to the event. Place added marks against the actual decoded cut. For enlarged views, recapture at the density required while preserving the intended CSS viewport and state; record raster and viewport separately and inspect the closest encoded view.

When compressing a wait, distinguish the initiating action, the cue that time is being compressed and the observed ready state. Compare spending the recovered time on the result. Added editorial motion must not look like evidence of progress the product did not show.

For a tutorial with separate presenter and screen tracks, measure their alignment and retain the source-to-playback offset. Click, screen-activity and transcript timelines can suggest where focus changes; the editor still chooses the subject. Remote editing is a separately selected route with its own access and effects, not a dependency of a local film.

## Worked example

Keep source seconds 12 to 17 and place them at master seconds 4 to 9. A word at source second 14 is at master second 6 before any later trim. Compare first and last words and check the decoded cut at joins. Keep original event time separate from the sampled frame where the UI first visibly changes.
