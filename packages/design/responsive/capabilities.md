+++
description = "Use it to remove scripts, graphics or hover in the actual environment and check that the task survives."
uses = [{ scope = "local", resource_id = "responsive" }]
see_also = [{ scope = "local", resource_id = "system/presentations" }]

[guide]
exposure = "internal"
+++

# Keep the task when a capability is absent

## Remove capabilities deliberately

Remove the capability in the actual environment. A viewport change alone does not remove scripts, graphics or hover.

| Remove or change | Check in the rendered result |
| --- | --- |
| Scripts | Navigate real pages and open built-in disclosures. Search or a calculation should use its working fallback or explain its unavailability. |
| Graphics | Block the image, canvas or shader. Preserve the facts and relationships in text or a table. An empty frame with a caption reading “chart” loses the content. |
| Custom fonts | Block the fonts on the target platform. Check whether the actual replacements preserve type roles, wrapping, numbers and clipping. |
| Hover | Tap each important control. Check for a stuck hover state and use the focus or touch path for anything the hover revealed. |
| Transparency | Use an opaque reading surface. Measure the actual text and focus contrast on it. |
| Increased contrast | Inspect weak dividers, secondary text and focus. Strengthen the parts the user needs to distinguish. |
| Forced colors | Let the system colors apply. Restore boundaries that depended on fills and check anything previously carried by a background image. |

Where supported degraded conditions combine, verify the combined outcome rather than assuming independent passes compose. Choose combinations from actual risk and prior failures. Apply the project's [Design each presentation and check how it is reached](../system/presentations.md) when choosing the combinations. Check the behaviors the task uses.

A presentation state and a missing capability answer different questions. A designed dark or reduced-motion form still needs its script, graphics and font failure paths; calling it a fallback does not establish those paths. For a publication promising static reading, the no-script document keeps its content and destinations. For a feature requiring computation, show its working alternative or honest unavailable state. Test graphics that fail after first load as well as an initially blocked asset, and keep the same information available when a graphics context is lost.

## Finish with the right observation

Use representative real pages and all previously failing cases in the affected scope. Check meaningful intermediate sizes as well as named verification widths; verification widths are not necessarily design breakpoints. Observe navigation, focus, local scrolling, Back and form completion where affected. Compare fine pointer at a narrow width with touch at a wider one. Device emulation is useful but does not establish an actual keyboard, browser chrome, performance or assistive-technology result on the target device. Record those gaps separately.

Return the corrected boundary, selected states and actual results. Keep unrun cases and owner decisions visible without creating a new universal matrix, fixture generator or wrapper. The project owns its layout system, supported platforms, acceptance criteria and tools.

Use the owning project's required combinations of states, contexts, widths, zoom, input, orientation and capabilities. Include representative page families and previously failing cases within that scope. A pass with scripts disabled in light does not establish the dark reduced-motion result, and a no-script pass plus a forced-colors pass does not establish their combination. Record the device, browser, orientation and tested settings with the observation, distinguishing actual device work from emulation.

## Worked example

Block storage, set dark in the system, then use the site’s explicit light choice. Check first paint, controls and navigation during the session. Reload and describe whether the choice survives. Then turn off script for a page that supports static reading. Its meaning and links should still work. A live calculation can report that its feature is unavailable and keep the explanation readable.

## Sources for the check

- [Media Queries Level 5](https://www.w3.org/TR/mediaqueries-5/) and [CSS Color Adjustment](https://www.w3.org/TR/css-color-adjust-1/): contrast, transparency and forced-color behavior.
- [MDN: forced colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/forced-colors) and [hover](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/hover): test preference and input paths on their own terms.
- [GOV.UK progressive enhancement](https://www.gov.uk/service-manual/technology/using-progressive-enhancement): a complete baseline and deliberate enhancement layers.
- [MDN: WebGL practices](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices): unavailable or lost contexts that need a useful fallback.
- [Chrome device emulation](https://developer.chrome.com/docs/devtools/device-mode): what simulation can check and where real-device observation remains necessary.
