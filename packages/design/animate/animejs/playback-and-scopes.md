+++
description = "Use it to change a running animation, wire scrolling to it or clean up a scope."
uses = [{ scope = "local", resource_id = "animate/animejs/api-v4" }]
see_also = [{ scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/animejs/tokens-reduced-motion" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/scroll/guide" }, { scope = "local", resource_id = "animate/feedback-roles" }]

[guide]
exposure = "internal"
+++

# Control anime.js playback and scopes

Change a running animation, wire scrolling or clean up a scope.

## 3. Playback settings, callbacks and methods

| Kind | Members |
| --- | --- |
| Playback | `delay`, `duration`, `loop` (number of repeats), `loopDelay`, `alternate`, `reversed`, `autoplay`, `frameRate`, `playbackRate`, `playbackEase` |
| Callbacks | `onBegin`, `onBeforeUpdate`, `onUpdate`, `onRender`, `onLoop`, `onPause`, `onComplete`, and a promise through `then()` |
| Methods | `play`, `pause`, `resume`, `restart`, `reverse`, `alternate`, `seek`, `complete`, `cancel`, `revert`, `reset`, `stretch`, `refresh` |

- **Interruption:** on new input, `reverse()` or `seek()` from where playback is; `restart()` jumps to the beginning and is a detection signal ([Reverse or retarget a control in flight, “Rules”](../retarget-controls.md#rules)).
- **Feel check:** `playbackRate` below one slows an animation or timeline for review; it never ships slowed ([Inspect an interface in its rendered context](../../inspect.md)).
- `complete()` settles at the end state (use when reduced motion switches on mid-flight); `revert()` removes the animation and restores the original inline styles: the way back to the still, when the enhanced figure gives way to its still. Follow the component’s teardown rules when it leaves the page.
- `autoplay: false` holds an animation until the island plays it, for example from `onScroll` or an intersection.
- Content and focus never wait for `then()` ([Choose feedback roles and timing, “2. Rules”](../feedback-roles.md#2-rules)).

## 4. Utilities and the engine

- `utils.set(targets, props)` applies values immediately: the right way to set an island's start state after mount. `utils.get`, `utils.remove` read and detach.
- `engine.pauseOnDocumentHidden` is on by default and keeps hidden tabs idle; leave it on.
- `engine.timeUnit` switches the whole page between milliseconds and seconds; leave it at the default so token conversion stays in one place, and never change it from one island because it affects every animation on the page.
- `engine.defaults` holds library defaults (including a default ease); islands pass token values explicitly instead of editing or relying on them.

## 5. Scopes and scroll observers

- `createScope({ root, defaults, mediaQueries })` groups every animation built inside its constructors under one `revert()` and re-runs them when a media query changes; `self.matches` reports each query. `addOnce()` runs a constructor once; `keepTime()` preserves playback position across re-runs. Pattern, including the motion-preference contract: [Use tokens and reduced motion with anime.js, “3. Reduced motion through a scope”](tokens-reduced-motion.md#3-reduced-motion-through-a-scope).
- `onScroll({ container, target, axis, enter, leave, sync, onEnter, onLeave, onUpdate })` passed as `autoplay` ties playback to scroll: `sync: true` scrubs, a play/pause pair toggles on crossing, a number smooths. Select and record response under [Design motion that helps people use the interface](../../animate.md) interruption and reversal; a scrub settles to the requested position ([Compose a sequence people can follow](../../films.md)). Scroll-linked playback is permitted only where scroll position is the island's meaning; the judgement and the browser alternatives are in [Build a scroll-linked effect or one-time arrival](../scroll/guide.md). Never ship `debug: true`.

## Worked example: reverse a panel in place

```js
import { animate } from "animejs";

const reveal = animate(panel, {
  x: [16, 0],
  opacity: [0, 1],
  duration,
  autoplay: false,
});
openButton.addEventListener("click", () => {
  setPanelOpen(true);
  reveal.play();
});
closeButton.addEventListener("click", () => {
  setPanelOpen(false);
  reveal.reverse();
});
```

`setPanelOpen` updates the logical state and focus when the input arrives. Keep the panel mounted through the exit, let a second open play forward from its current time, and remove both listeners when the component leaves.

## Sources

- [anime.js documentation: Animation](https://animejs.com/documentation/animation): targets, tween values, keyframes, playback settings, callbacks and methods.
- [anime.js documentation: Scope](https://animejs.com/documentation/scope): `createScope` media queries, `self.matches` and `revert()`.
- [anime.js documentation: Engine](https://animejs.com/documentation/engine): `pauseOnDocumentHidden`, `timeUnit` and engine defaults.
- [anime.js documentation: onScroll](https://animejs.com/documentation/events/onscroll): scroll observer thresholds, sync modes and callbacks.
