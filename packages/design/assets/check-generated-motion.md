+++
description = "Use it when a shot is generated from an image, to check the starting image and to control motion, identity and the join of first and last frames."
uses = [{ scope = "local", resource_id = "assets" }]
see_also = [{ scope = "local", resource_id = "films/check-a-render" }]

[guide]
exposure = "internal"
+++

# Make and check a generated shot

For image-to-video, check the starting image before generating. Ask the generator to move the subject, camera or environment where the shot needs it. Keep the source's identity, composition and light. Starting-frame control, general appearance conditioning and a controlled ending give different results. Give the first and last frames together when the model supports both and the join depends on them.

One clear subject action and one camera move, or a locked camera, can reduce drift in a generated shot. Describe purposeful environmental movement separately. If a long request changes a face, material or camera on its own, split it into short shots or narrow the uncontrolled motion. Keep the action that explains the beat. An authored sequence can coordinate as many parts as it can make readable.

| Check | What you should see |
| --- | --- |
| Identity and continuity | The source's features, material and light survive the agreed change. |
| Action and camera | The action stays legible, with no unasked drift, jump or deformation. |
| Endpoints | The cut, match or deliberate contrast connects the actual frames. |
| Words | The intended text remains readable through its full interval. |
| Clock | A reversible scrub still conveys the claim. A linear story can depict irreversible change. Its frames must still be reproducible at a seek if the renderer requires it. |
| Safety and loop | Inspect flashes and sudden large movement. Where a loop is intended, check the seam, light, motion and sound. |

Watch at intended speed, then step through generated frames for transient errors. Compare the opening and ending to their neighbors. Matching endpoints cannot establish the middle. Keep the shot check with the generated asset and link the asset from the editable production record. Use the [Watch the delivered film and inspect difficult joins](../films/check-a-render.md) for the joined export.

## Worked example

A generated door opens onto a second scene. The start plate passes the still check and the end uses the same door silhouette as the next shot. At speed the motion looks coherent. Stepping through reveals a second handle appearing during a camera push. Lock the camera and regenerate the short door movement. Inspect the revised middle and the matching end before putting the shot back into the cut.

- [Veo prompting guidance](https://cloud.google.com/vertex-ai/generative-ai/docs/video/video-gen-prompt-guide)
- [Generate with first and last frames](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/video/generate-videos-from-first-and-last-frames)
- [WCAG flash threshold](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html)
