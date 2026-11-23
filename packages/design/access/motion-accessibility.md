+++
description = "Use it when a product offers system or explicit motion preferences, to check each route, its precedence, first paint and navigation, and honest persistence."
uses = [{ scope = "local", resource_id = "access/web" }]

[guide]
exposure = "internal"
+++

# Keep the task available when motion changes

## Choose the motion and task to check

Keep feedback and meaningful end states available when movement is reduced. If the product offers system preference and explicit choice, verify each route, their declared precedence, first paint, navigation and a change during motion. Test blocked persistence honestly; do not silently claim a choice was remembered. The motion method owns choreography and interruption. Page animation, autoplay backgrounds, an authored film and the player's controls have different contracts; a reduced-motion preference is not permission to erase an encoded story.

Choose observations from the affected task and the project's supported matrix. Use actual keyboard and assistive-technology interaction alongside a scan, naming the platform and versions. Source markup, emulated preferences and screenshots prove different things. A screenshot cannot establish an announcement, and a scan cannot establish that the task can be completed. Keep unrun paths visible. Use the project’s supported browsers, design values and conformance requirements.

## Reduce movement and keep the relationship legible

Test both the system preference and the page control when it offers one. With Follow system active, change the system setting. Then keep the system unchanged and choose Reduce in the page. Compare the same resting state and focus. Repeat while a control is opening, a field is running or a sequence is scrubbed. In-flight travel should resolve to a useful state, and later work should use the current preference. Inspect the initial render before any preference script runs.

Translate displacement, zoom, parallax and rotation into a direct state change, a still comparison or a quiet non-spatial cue. Keep text, navigation and the user's current position. For a story driven by scroll, the ordered stills and labels should make the same claim. For background animation, test the actual pause control and observe the same frame when returning from another task. Check the applicable pause, stop, hide and flashing rules even when the user has not asked for reduced motion. A preference switch cannot make a harmful default acceptable.

For [Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html), automatically starting movement, blinking or scrolling that lasts more than five seconds alongside other content needs a pause, stop or hide mechanism unless essential. Automatically updating information alongside other content needs such a mechanism or control over update frequency; the five-second condition does not apply to that branch. Check the actual activity before claiming the essential exception.

For [Three Flashes or Below Threshold](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html), keep flashes to no more than three in any one-second period, or establish that they fall below both the general and red-flash thresholds. Count across loop seams and use the presented area and luminance changes; a sampled still cannot establish this.

## Check interruption, suspension and preference changes

Open and close a control rapidly, reverse it halfway through travel and inspect the settled state. Focus must reach the usable control when its interaction requires it; an animation must not delay or later move focus. Scroll a moving region off screen, hide the tab and return. Confirm that suspended work resumes under the current preference without a large time jump or an accidental reset of the person’s position.

If the page offers a remembered preference, navigate and reload under each setting and inspect the first paint for a flash of the other presentation. Block storage and check the stated fallback: a page-only choice should remain usable and the control should explain that it will not be remembered. Test the system route and explicit choice separately, including a change while motion runs, in each supported color mode.

Screen expressive light and flicker across the actual frames, including loop seams, at the intended display size. Use the applicable flash thresholds; a quiet-looking sampled still cannot establish the safety of the sequence. Put the preference source, page, width, actual observations and unrun cases in the existing review or reply. Keep an automated regression in the maintained test suite where one can exercise the defect.

[Animation from interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html) explains disabling interaction-triggered motion. [The reduced-motion media feature](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion) explains the system preference, and [Web Storage](https://html.spec.whatwg.org/multipage/webstorage.html) describes unavailable storage and its errors.

## Worked example

Open a disclosure under the system’s reduced-motion setting and through the site’s explicit setting if it offers one. Confirm both routes reach the expected first paint. Toggle during entrance: the new state and the focus path still work. For a moving field beside text, test its pause or stop control where required. Check autoplay, flashing and interaction-triggered motion against the applicable [WCAG criteria](https://www.w3.org/TR/WCAG22/). Test an encoded film’s captions and player separately from the page’s transition into the player.
