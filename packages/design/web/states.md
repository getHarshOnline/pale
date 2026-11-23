+++
description = "To give a built-in control its input states, or to animate an existing disclosure without a second state machine."
uses = [{ scope = "local", resource_id = "web" }]
see_also = [{ scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "responsive/viewport" }, { scope = "local", resource_id = "responsive/capabilities" }]
official_sources = [{ label = "Apply input states and transitions in CSS", url = "https://www.w3.org/TR/css-transitions-2/" }]

[guide]
exposure = "internal"

[stack]
+++

# Apply input states and transitions in CSS

Use this overlay to give a built-in control its input states, or to animate an existing disclosure without owning a second state machine.

Choose hover styles with the actual pointer’s capabilities. Keep `:focus-visible` meaningful on every ground, independent of hover. Use `:active` for the press drawing. Commit an ordinary action on its normal activation event so moving away can cancel. Leave labels and hit regions in place while the drawing moves.

Use `:focus-within` when a group needs to show that one of its controls has focus. Use `:has()` for a supported relationship such as a container reflecting a child’s checked or invalid state, rather than mirroring that state with another script flag. Keep the control’s own focus indication and a useful fallback. Feature support in the browser and acceptance by the stylesheet pipeline are separate checks.

## Worked example: transition an existing modal dialog

```css
.shell__dialog {
  opacity: 0;
  transition-property: opacity, transform, display, overlay;
  transition-duration: var(--duration-leave);
  transition-timing-function: var(--easing-productive);
  transition-behavior: allow-discrete;
}
.shell__dialog[open] {
  opacity: 1;
  transition-duration: var(--duration-enter);
}
@starting-style {
  .shell__dialog[open] {
    opacity: 0;
  }
}
```

A dialog enters the top layer through `showModal()`. The built-in open state drives the surface above. `@starting-style` supplies the first entry frame. A discrete `display` or `overlay` transition can keep the exit visible until it completes. Let focus and the application state follow the dialog contract as soon as input arrives. When the stylesheet pipeline or browser cannot use this feature, the dialog should still open, close and restore focus with an immediate change. Use `:popover-open` where the disclosure uses a popover. Check both supported mechanics in the rendered browser.

## Use a CSS spring with a complete transition fallback

```css
.drawer {
  transition: transform var(--duration-sequence) var(--easing-productive);
}
@supports (transition-timing-function: linear(0, 1)) {
  .drawer {
    transition: transform var(--motion-spring-productive-duration)
      var(--motion-spring-productive-easing);
  }
}
```

Guard the substitution with `@supports`. An invalid computed custom-property value can invalidate a declaration that already won the cascade. The supported default transition is usable on its own. Test a reversal and the reduced-motion state separately. See [Reverse or retarget a control in flight](../animate/retarget-controls.md) and [Make the browser task accessible](../access/web.md).

## Give hover, focus and the shortcut separate jobs

```css
.command-trigger__chord {
  display: none;
}

@media (hover: hover) and (pointer: fine) {
  .action:hover {
    border-color: var(--color-strong-line);
  }
  .command-trigger__chord {
    display: inline;
  }
}

.action:focus-visible {
  outline: var(--focus-width) solid var(--color-focus);
  outline-offset: var(--focus-offset);
}
```

The button stays visible at every width. The chord is a hint for the desk, and hover changes a different visual part from focus. Tab to the control with a pointer still over it. Both should read. On touch, verify that tapping does not leave a misleading hover.

Draw focus with an outline or another tested indicator that survives forced colors; a box shadow alone disappears there. Keep the focused control's indicator when a surrounding group also uses `:focus-within`. Prefer a stable hit box with an animated child when the drawing's transform would move it away from the pointer. Hold the pointer at the edge: the state must not alternate because its own motion repeatedly ends hover.

## Finish touch and interrupted input

Suppress `-webkit-tap-highlight-color` only where the control already paints its own prompt press feedback. Scope `user-select: none` to control chrome that needs it; keep prose, code and useful values selectable. On an appropriate control, `touch-action: manipulation` permits panning and pinch zoom while disabling double-tap zoom. Do not apply it indiscriminately to reading content.

A custom drag owns one pointer and ends on release, `pointercancel`, `lostpointercapture` or loss of the active window. Use pointer capture where the drag must continue outside its element, ignore unrelated pointers, and make cleanup safe when more than one ending event arrives. Declare gesture handling before the gesture begins: for a horizontal drag that preserves vertical page scrolling and pinch zoom, consider `touch-action: pan-y pinch-zoom` on that surface. Exercise cancellation and a second drag without reloading. Keep the keyboard and single-pointer alternatives required by [Make the browser task accessible](../access/web.md).

Use `overscroll-behavior: contain` on a scrolling dialog region where reaching its end should not scroll an ancestor. Verify the actual scroll container; this is not a substitute for the modal's background, focus or restoration behavior.

For full-height regions, device insets, keyboard opening and platform auto-zoom, use [Design for the visible viewport and its input](../responsive/viewport.md). Where a project places regions by context, test fine and coarse input at both narrow and wide widths. Its conditions must cover every supported reader without hiding a required destination in both paths. Keep the project's exact context attributes and print policy with that project.

## Find conflicting state mechanics

| Symptom | Inspect and correct |
| --- | --- |
| Hover wins while a control is held down | Check specificity and rule order; make the press visible on contact without moving surrounding layout. |
| Focus vanishes in forced colors | Replace shadow-only focus with a tested outline or border; retain non-color state cues. |
| A dialog flickers or snaps | Use one owner for its opacity and transform. Keep phase attributes for other parts; check `@starting-style` after equal-specificity open rules and test entry, exit and rapid reversal. |
| Unrelated dimensions animate | Name transition properties instead of `transition: all`; measure any intended layout morph. |
| A reduced treatment loses the answer | Put each reduced rule beside the motion it translates; exercise both the system preference and any explicit preference route. |
| Touch remains stuck after interruption | Check pointer identity, cancellation and capture cleanup, then start another gesture. |

## Sources

- [MDN pointer events](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events): pointer identity, capture and cancellation.
- [MDN `touch-action`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/touch-action): which pan and zoom gestures the browser retains.
- [MDN `overscroll-behavior`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/overscroll-behavior): scroll chaining and the scroll-container requirement.
- [MDN forced colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/forced-colors): replaced colors, suppressed shadows and system color choices.
- [MDN `@starting-style`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@starting-style) and [transition behavior](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/transition-behavior): entry cascade order and discrete exit transitions.
