+++
description = "Use it to choose between a source check, a browser capture and keyboard, pointer or assistive technology for the behaviour you need to see."
uses = [{ scope = "local", resource_id = "inspect" }]

[guide]
exposure = "internal"
+++

# Choose a source check or browser capture

Use a source check to catch detectable mistakes in the files under review. Use a browser capture to inspect a delivered interface at its actual URL and dimensions. Use keyboard, pointer and assistive technology for the behavior that measurements and stills cannot show.

The Pale checkout supplies the commands under `tools/design-verification`. The calling project supplies the files, URLs, browser settings and any extra design rules.

| Task | Tool |
| --- | --- |
| Check empty link destinations, markup and obvious CSS problems | `static_audit.py`. Enable token-role, width or template rules only when they apply. |
| Check links or caller-supplied disclosure terms in written guidance | `check_sources.py`. Pass the files and their root. |
| Record and inspect actual browser views | `capture.mjs`. Pass the resolved viewport and target values, plus the project’s preference mapping when needed. |

## Read what the capture measured

The capture verifies the applied viewport and preferences before reporting a cell. Check requested and final URLs after redirects. The main response and PNG have separate hashes. Use `--record-responses` to record observed same-origin HTTP GETs for comparison with the served build. A build label alone cannot show which response the browser read.

Use `--no-js` and `--forced-colors` for the supported capability paths. With `--scrub N`, the tool samples equally spaced offsets from both directions. Visit the authored holds separately. Use a live check for motion and for a cell whose measurements do not settle. Geometry findings are based on DOM boxes. Inspect actual hit areas, overlapping targets, and exclusions before deciding whether a control meets its target. For a crop, keep its parent capture and the viewport, preferences, platform and build that produced it.

## Worked example: check a changed form

Run a source check on the relevant templates, then capture the real form at a narrow width. A missing image description appears in the source findings. A control that runs off the page appears in the capture. Check keyboard focus and error recovery yourself in the form.

```sh
python3 tools/design-verification/static_audit.py --json path/to/templates
node tools/design-verification/capture.mjs --profile path/to/profile.json \
  --out path/to/captures --widths phone --states light --build local-preview \
  http://127.0.0.1:8000/contact
node tools/design-verification/capture.mjs --verify-captures path/to/captures/cells.json
```

A skipped check leaves the question open. The capture report gives the browser, preferences, measured conditions and output files. Read the images and check the actual input and recovery paths before judging the form.
