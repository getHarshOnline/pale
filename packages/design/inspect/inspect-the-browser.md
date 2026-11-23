+++
description = "Use it to choose a browser route by the claim it can prove and to read the capabilities it actually exposes."
uses = [{ scope = "local", resource_id = "inspect" }]

[guide]
exposure = "internal"
+++

# Use the review browser for what it can observe

## Establish what the route can observe

Choose an allowed browser, inspection integration or established capture tool by the claim it can prove. Read actual exposed capabilities. Preference emulation, navigation, touch dispatch, pointer hover, accessibility inspection, cascade inspection and performance tracing are separate abilities. A narrow viewport supplies no touch input. Confirm applied preferences and actual CSS viewport in the session that performs the task. An unavailable or denied route leaves its dependent claims open; it does not justify bypassing the host or creating a substitute production task.

Record the operating system, browser/version, device pixel ratio, viewport, input route and build. When the target review platform is unavailable, label the available platform and leave dependent font, rasterization and native-control claims unproved. Check the face that actually rendered. Record injected content or extensions that compromise an isolated view.

## Look and exercise the result

Reach the emitted page by its real URL and relevant neighboring navigation. Leave through its own controls, use Back, and inspect the returned context. Include a related surface or identity crossing when the change reaches it. Injected markup or an artboard cannot establish routing or history.

Wait for fonts and required images, visit below-the-fold content, then confirm that the capture contains the intended complete document. A lazy image, scroll snap, growing page or redirect can invalidate an apparently full capture. Record requested and final URLs. Keep native-scale captures, their exact identities and actual measurements; read the images before reporting what they show. Saving screenshots alone is not inspection.

Watch motion moving. A stable still can support hierarchy or alignment; freezing an animation conceals timing, interruption and transition defects. For position-driven work, visit relevant holds from both directions and check the settled result. Separate ambient, reader-position and film clocks. Keep controls, focus and capability behavior in their actual host.

After a rebuild, refresh the already-open reader view and verify its loaded styles and result. A fresh test tab can differ from a stale live preview. Confirm measured width after resizing or emulation, and clear temporary overrides before returning the view to its reader.

Keep device pixel ratio or capture scale separate from the CSS viewport: a higher-resolution image is not a wider layout. Freeze motion only for a labelled still inspection. Resume or reload into the normal running state before checking timing, interruption or a loop. When the task includes a scripts-off path, enter through a real link and exercise that path’s navigation and Back rather than inferring it from an image.

## Make the capture conditions observable

With Playwright, set media preferences through `page.emulateMedia()` and confirm the intended queries with `matchMedia()` in the page. Test explicit site choice and system preference as separate routes. Set the CSS viewport and device scale independently, then read back the viewport and `devicePixelRatio` instead of typing the intended values onto the capture.

Before a still, await `document.fonts.ready`, inspect the faces that actually rendered, and scroll through lazy content. For each required image, check `complete` and a nonzero `naturalWidth`; a failed image can also report completion. Recheck content that appears after interaction. A settled font-loading promise alone does not prove that the intended face arrived.

Use screenshot `animations: "disabled"` only for an identified still comparison: it advances finite CSS/Web Animations to their end and resets infinite ones for the capture. It does not establish the moving result. Open the same URL in a context with `javaScriptEnabled: false` for a scripts-off check; deleting script elements after page load does not exercise that delivery path.

[Playwright's page API](https://playwright.dev/docs/api/class-page) defines media emulation and screenshot animation handling; [browser contexts](https://playwright.dev/docs/api/class-browser#browser-new-context) define scale and JavaScript settings. [FontFaceSet.ready](https://developer.mozilla.org/en-US/docs/Web/API/FontFaceSet/ready) describes when font loading and its layout work have settled.

## Diagnose a failed browser check

When a click times out, inspect the pointer hit and navigation before changing the page. The combined box of a wrapped inline link can put its center in whitespace. Find a point inside a visible client rectangle, confirm it hits the anchor or a child, then observe the resulting URL and any new tab. Calling a DOM click can check the handler. It does not test whether a real pointer can reach the link.

A swipe can update the current marker while its object is still moving. Record the input route, axis and final offset. Test the follow-up tap when it settles. Test input during travel separately if interruption matters. For a scrubbed sequence, compare an authored hold with the same position reached from above and below. An evenly spaced sample can miss that hold. If scroll snapping prevents reaching it, inspect the actual resting position and keep the unresolved sample visible.

## Worked example

Open a shared link in the review browser and record where it lands. Change the viewport, then read back its CSS dimensions. Use an actual touch path for a touch claim. Wait for fonts and lazy images, scroll through the page and inspect the result. After a rebuild refresh the tab the reader uses and check the loaded styles. Compare it with a new tab if they differ.
