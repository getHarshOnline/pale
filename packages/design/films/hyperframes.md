+++
description = "When the project chose HyperFrames for an authored timeline."
uses = [{ scope = "local", resource_id = "films" }]
official_sources = [{ label = "Check and render a local HyperFrames composition", url = "https://github.com/heygen-com/hyperframes" }]

[guide]
exposure = "internal"

[stack]
+++

# Check and render a local HyperFrames composition

Use this overlay when the project chose HyperFrames for an authored timeline. Use the installed guide and CLI version the project binds. Keep scenes editable and timed by the composition clock. Give a scene an explicit duration, stable IDs and a paused timeline that can seek. Keep user input history, wall-clock time and network requests out of visible frames.

The workspace's film engine supplies a launcher for the upstream CLI, along with the local binary, Node, Chromium, media tools and upstream skill checkout. Read the film and the source references before preparation.

## Worked example: check a short sequence before exporting

Run the launcher's `check` on the film with JSON output, then its `render` to an MP4 file, both through the workspace binding. Probe the render's streams before review:

```sh
ffprobe -v error -show_streams path/to/render.mp4
```

Read lint, runtime, layout and contrast separately. If browser inspection was skipped, fix the missing tool or condition before treating those checks as run. Compare cold seeks and ordinary playback. Open decoded frames at the joins and play the final file with sound when it has sound. The guarded local render supplies the file and its technical result. Use the film’s own review and delivery path for its creative and release decisions.
