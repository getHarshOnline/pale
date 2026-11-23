+++
description = "Use it when a motion fault shows in the browser, to find a change to try and a way to check it."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "user-task" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/feedback-roles" }, { scope = "local", resource_id = "animate/preserve-transform-origin" }, { scope = "local", resource_id = "animate/frequency-and-pending" }]

[guide]
exposure = "internal"
+++

# Diagnose motion in the browser

Use the section for the fault you see. Each gives a change to try and a way to check it. The rules live in [Give control motion a clear job](productive-motion.md), [Show what happens first](temporal-hierarchy.md), [Keep motion responsive when its input changes](interruption-reversal.md), [Give expressive motion a purpose](expressive-motion.md) and [Implement motion from the shared design](motion-implementation.md); these tips apply them. The token names illustrate one project’s scheme; use the consuming system’s roles. Keep the check result in the component’s existing review or maintained test, and stop once the fault is resolved.

## Contents

- Reviewing a motion that feels off
- Press, arrival and origin
- Previews seen in a row
- Timing by frequency and speed
- Choosing the curve
- Designing the reduced state
- Fixing rendering glitches

## Reviewing a motion that feels off

### Slow it down and step through it

A wrong origin, two properties out of step or an overshoot that reads as a wobble are invisible at full speed. Open the browser's animation inspector, replay the motion at a slowed rate and drag the playhead through entry, exit and a mid-flight reversal; where the inspector cannot catch it, record the screen and step the recording frame by frame. **Check:** the first frame starts from where the element was, coordinated properties follow their intended phases, and the last frame reaches its intended rest or loop join. **Read:** [Show what happens first, “Prototype, don't board”](temporal-hierarchy.md#prototype-dont-board); [Inspect an interface in its rendered context](../inspect.md); [Check a task from entry to result](../user-task.md).

### Reverse it halfway before accepting it

An interrupted CSS transition starts again from the current value and shortens its reversal; a replaced keyframe animation restarts from its first frame. Trigger each reversible motion, reverse it mid-flight, then press the trigger rapidly several times. **Check:** the element turns around from where it is, with no jump and no queue; the implementation retains the current position and direction rather than replacing keyframes from their beginning. **Read:** [Reverse or retarget a control in flight, “Choosing mechanisms”](retarget-controls.md#choosing-mechanisms); [Carry a design through the web build](../web.md).

## Press, arrival and origin

### Answer the press on contact

Acknowledgement on pointer-down tells the reader the interface heard them before any result arrives. The control's drawing scales to `scale.press` on `duration.feedback`, from `:active` and its actual interaction state; under reduced motion it reads `motion.reduced.press-scale` and emphasis or a tone step carries the press. Build it in [Carry a design through the web build](../web.md). **Check:** force the pressed state in DevTools and see it paint; press and slide off, and the press shows while nothing activates; in source, the press drawing uses the shared role while the hit area stays stable. **Read:** [Choose feedback roles and timing, “1. The role contract”](feedback-roles.md#1-the-role-contract); [Design controls that work through the whole task](../controls.md); [Carry a design through the web build](../web.md).

### Arrive from a visible shape, never from nothing

A surface that grows from zero size appears from nowhere; an object that already has its shape reads as coming forward. Let the container hold its footprint and arrive by opacity; never `scale(0)`, zero height or a point. A productive entrance scales from `scale.enter` with opacity; an expressive arrival may add the world's depth (`world.depth.blur`). **Check:** in a slowed entrance the first visible frame already shows the surface at its place and near its size; in source, grep `scale(0` and zero-height start states. **Read:** [Choose feedback roles and timing, “2. Rules”](feedback-roles.md#2-rules); [Give expressive motion a purpose, “What it may express”](expressive-motion.md#what-it-may-express); [Choose what material and light communicate](../surfaces.md).

### Grow from the trigger

A popover that scales from its own centre while its trigger sits at a corner breaks the relation it exists to show; the error is plainest when both axes differ. Compute `transform-origin` from the trigger's position relative to the surface in the module that opens it, and write it on the surface element; a centred dialog keeps its centre. Follow the project’s component mounting rules. **Check:** open a corner trigger at the narrowest width with playback slowed; the surface grows out of the trigger, and DevTools computed `transform-origin` points at the trigger's side. **Read:** [Build the origin and reduced form into the control, “3. CSS first”](preserve-transform-origin.md#3-css-first); [Carry a design through the web build](../web.md).

## Previews seen in a row

### Make the second preview instant

The intent delay stops accidental hover from opening previews; once the reader is scanning neighbouring previews, the delay and the entrance both stand in the way. The preview module keeps a warm state after one preview opens; while warm, the next trigger opens without `delay.preview` and without its entrance, marked by a class the module sets. The warm state lasts while a preview is open and for `delay.preview-warm` after it closes. Keyboard focus never waits for the delay. **Check:** hover across three adjacent triggers: the first waits and enters, the next two appear at once; tab through the same triggers and each opens on focus. **Read:** [Choose feedback roles and timing, “1. The role contract”](feedback-roles.md#1-the-role-contract); [Make the browser task accessible](../access/web.md); [Design controls that work through the whole task](../controls.md).

## Timing by frequency and speed

### Choose the role by its meaning

Perceived speed follows motion speed: the same wait feels shorter when the motion answering it is brisk, and interface motion should keep use immediate. Assign the semantic role first and use its token; do not relabel a dismissal as feedback merely to shorten it. If the role cannot express required distance or scale, show the token owner the movement and the distance the role needs to cover. **Check:** in the slowed capture the change is legible; the component uses the appropriate shared role, or its authored scene timing. **Read:** [Show what happens first, “Duration encodes importance and distance”](temporal-hierarchy.md#duration-encodes-importance-and-distance); [Choose feedback roles and timing, “1. The role contract”](feedback-roles.md#1-the-role-contract).

### Drive it from the keyboard at key-repeat speed

Keyboard actions repeat hundreds of times; any motion on them turns speed into waiting. Hold an arrow key through a result list and fire the shell chord repeatedly. **Check:** the highlight never trails the key repeat, result content never animates, and the shell is present on the first frame when summoned by chord. **Read:** [Keep repeated and pending responses useful, “3. Frequency rule”](frequency-and-pending.md#3-frequency-rule); [Design controls that work through the whole task](../controls.md).

### Use it many times before keeping a hover motion

A hover effect pleasant once becomes noise when met dozens of times in a session. Before accepting a transition on a frequent control, walk a real task on the page that crosses the control many times; if the motion registers as waiting or flicker, cut it to the immediate state change. **Check:** after the walk, DevTools computed style shows the control either with no transition or on `duration.feedback`, and the existing review explains any remaining frequent movement. **Read:** [Keep repeated and pending responses useful, “3. Frequency rule”](frequency-and-pending.md#3-frequency-rule); [Carry a design through the web build](../web.md).

## Choosing the curve

### Enter and leave on the decelerating token curve

For a system built around a decelerating response, a fast start answers the input promptly. Use its shared entry and exit curves, including an accepted asymmetric pair where one exists. The example names a single curve `easing.productive`. **Check:** the animation inspector shows the intended curve and the interface answers input immediately; local literals have not silently replaced a shared role. **Read:** [Choose feedback roles and timing, “2. Rules”](feedback-roles.md#2-rules); [Show what happens first, “Out faster than in”](temporal-hierarchy.md#out-faster-than-in); [Carry a design through the web build](../web.md).

### Move between two positions on the symmetric curve

An element moving between two visible positions, a sliding indicator or a reordered item, is on screen at both ends: a decelerating curve makes it lurch away from rest. It takes `easing.move` on its role's duration; entrances and exits keep `easing.productive`. **Check:** in source, every indicator or reorder transition reads `var(--easing-move)` and no literal curve; the animation inspector draws a symmetric curve. **Read:** [Choose feedback roles and timing, “2. Rules”](feedback-roles.md#2-rules).

## Designing the reduced state

### Play the reduced form beside the full one

A reduced state tested only for what stopped can pass while the page goes inert and its peak disappears. Open the page twice, one tab emulating `prefers-reduced-motion: reduce`, and run each motion in both at slowed playback: every role lands on its `motion.reduced.<role>` step, and each expressive moment changes its light or material in place on the world's `world.motion.reduced.*` tokens. **Check:** the reduced tab shows every state change, no element at a start state, the page's peak still distinct from its baseline, and immediate alternatives still settle content, focus and cleanup correctly. **Read:** [Design the reduced-motion alternative, “Translate, don't delete”](reduced-motion.md#translate-dont-delete) and "The peak without travel"; [Make the browser task accessible](../access/web.md).

## Fixing rendering glitches

### Reach for blur last, and only from a token

When a crossfade shows two distinct objects, a brief blur while crossing can bridge them into one transformation; its cost climbs with the radius and the area of the layer. Try easing and role first; then blur on the smallest layer that carries it: a productive crossfade passes through `blur.productive` (reduced: `motion.reduced.blur`), and an expressive moment of a world with depth blurs by `world.depth.blur`. **Check:** slowed, the two states read as one change; a performance trace of the running motion shows its frame cost, summarized in the existing review. **Read:** [Give expressive motion a purpose, “What it may express”](expressive-motion.md#what-it-may-express); [Carry a design through the web build](../web.md); [Choose what material and light communicate](../surfaces.md).

### Add `will-change` only for a shift that was seen

An element can shift by a pixel at the start or end of a transform animation as rendering hands it between layers; the hint keeps it on its own layer, and every layer costs GPU memory. Only after a slowed capture shows the shift, set the hint on the moving element when the motion starts and remove it when it ends. **Check:** the start and end frames line up with the resting position; in source, no resting rule declares `will-change`. **Read:** [Build the origin and reduced form into the control, “3. CSS first”](preserve-transform-origin.md#3-css-first); [Carry a design through the web build](../web.md).

## Worked example

A row preview opens on hover and keyboard focus. On the first opening, watch where the preview grows from. Then move focus through three rows at key-repeat speed. Give the first opening its useful spatial cue and update the next two immediately. Reverse halfway through the first opening and try the reduced form. If either leaves a stale row highlighted, repair how the preview follows the current target before adjusting duration.

## Sources

- [7 practical animation tips](https://emilkowal.ski/ui/7-practical-animation-tips): press acknowledgement, never entering from zero scale, instant subsequent tooltips, ease-out for entry and exit, origin-aware popovers, short durations and blur as a bridge between states.
- [You don't need animations](https://emilkowal.ski/ui/you-dont-need-animations): frequency as the test of whether to animate, keyboard actions left unanimated, and perceived speed.
- [Good vs great animations](https://emilkowal.ski/ui/good-vs-great-animations): frame-by-frame and slow-motion review, and ease-in-out for on-screen movement.
- [Agents with taste](https://emilkowal.ski/ui/agents-with-taste): the stance only: `will-change: transform` as the fix for a shaky transform, qualified here by its memory cost.
- [Base UI: Tooltip](https://base-ui.com/react/components/tooltip): the grouped delay after which adjacent tooltips show instantly, the instant state on focus and dismiss, and the anchored transform origin.
- [Base UI: Animation handbook](https://base-ui.com/react/handbook/animation): transitions preferred over keyframes because they can be interrupted, and the transform origin at the anchor.
- [CSS Transitions Level 1](https://www.w3.org/TR/css-transitions-1/): an interrupted transition starting from the current value and the reversing shortening.
- [Chrome DevTools: Inspect animations](https://developer.chrome.com/docs/devtools/css/animations): slowed playback, scrubbing the playhead and replaying animation groups.
- [Motion: web animation performance tier list](https://motion.dev/magazine/web-animation-performance-tier-list): blur cost rising with radius and layer size, and layers that exhaust GPU memory.
- [MDN: will-change](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/will-change): setting the hint just before a change and removing it after.
- [MDN: transform-origin](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/transform-origin): the origin a scale grows from and its keyword and length values.
- [MDN: easing-function](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/easing-function): the keyword curves and `cubic-bezier()` the token replaces.
