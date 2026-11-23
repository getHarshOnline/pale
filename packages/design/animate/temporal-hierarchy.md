+++
description = "Use it to decide what becomes visible when, so order expresses cause, comparison, anticipation or a shift of attention."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/frequency-and-pending" }]

[guide]
exposure = "internal"
+++

# Show what happens first

Time is a hierarchy axis. Order can express cause, comparison, anticipation or a shift of attention. The intended relationship decides the choreography. For interface feedback, durations are the productive roles in [Give control motion a clear job](productive-motion.md); what may move at a peak is [Give expressive motion a purpose](expressive-motion.md).

## Contents

- What becomes visible when
- Order encodes causality
- Duration encodes importance and distance
- Out faster than in
- Cascades
- Simultaneity
- Prototype, don't board

## What becomes visible when

This worked example follows a reader through a publication with previews, a command surface and a figure. Use the order to show how its capabilities become available. A film uses its own edit, and an ambient figure follows the clock chosen for it.

| Moment | What becomes visible | Motion kind |
| --- | --- | --- |
| Rest | The authored document, complete; state-bearing light where state is held | None |
| Hover or focus on a relation | The relation becomes legible (the link, its target world, its kind) | Productive acknowledgement or preview |
| Activating a crossing | The target world's signature (mark, underline geometry, and material at the crossing's prominence) announces itself; never the accent alone (see [Choose a visual direction from the actual brief](../art-direction.md)) | Productive timing |
| Summoning the command surface | The command world materialises over the still page | Productive shell entrance |
| Opening a working tool on the document (notes, citation) | The document becomes a workspace; the reading position holds | Productive handoff |
| Reaching an island or peak | The island expands into its moment | Expressive, once |
| Scrolling through a cinematic sequence | The idea arrives beat by beat at the reader's own pace, and holds when the reader stops ([Compose a sequence people can follow](../films.md)) | Expressive, driven by position |
| Scrolling on or dismissing | Everything settles back to reading | Productive exit, shorter than entry |

Use this example for the capabilities the page actually has. Avoid unrelated changes competing for attention and make each return to reading clear. A deliberate ambient treatment may advance on its own clock; a control answers the user. A project that assigns numeric strength levels should apply its own range here, rather than treating this example as a universal gate. [Give expressive motion a purpose, “Opportunity sweep”](expressive-motion.md#opportunity-sweep) helps find useful opportunities without inventing missing steps in the visit.

## Order encodes causality

When motion claims causality, its order must support that claim. These rules guide interface feedback; other temporal forms use [Compose a sequence people can follow](../films.md).

- **Cause before effect.** The trigger responds first (press, highlight), then the thing it opens or changes. Detect an effect that starts before its trigger visibly responds.
- **Container and content stay related.** Establish where content belongs as it arrives. Clipping, masking and stagger may overlap its container's transformation; unintended escape or competition is the defect, not simultaneous movement itself.
- **Subject before context.** When two things change together, the one the user acted on leads; secondary adjustments follow.
- **Replacement preserves focus.** Coordinate departure and arrival so the old and new views do not compete. A shared element may persist and transform while its contents change.

## Duration encodes importance and distance

- Larger travel, larger surfaces and higher-level changes take longer duration roles; small, local changes take the shortest.
- Detect a small local change using a long token, or a whole-view change using the shortest.
- Productive duration steps use their token roles. Authored scene timing follows the action in the scene and read window; classify it through [Implement motion from the shared design](motion-implementation.md).

## Out faster than in

This asymmetry belongs to productive control entry and dismissal. A film's exit or reverse move can take a different duration when its editorial purpose calls for it.

- A dismissal usually takes less time than an entrance because the user has already decided to leave. Use the project’s exit role and explain an intentional exception in the component specification.
- Keep the system’s chosen entry and exit curves. One system uses the same decelerating curve with different durations; another uses an accepted asymmetric pair. Inspect responsiveness and continuity rather than imposing a curve vocabulary. [Keep repeated and pending responses useful, “5. Open questions”](frequency-and-pending.md#5-open-questions) belong with the system owner.
- Dismissal returns control to the caller at the start of the exit, not after the presentation finishes.

## Cascades

- A cascade can express order or direct attention through a grouped change. State its purpose; arbitrary delay across a list does not become information.
- Keep the whole cascade inside the duration the equivalent single change would take; detect a list whose last item arrives noticeably later than a single item would have.
- Keep search results, settings and keyboard-navigable lists immediately usable; a staged entrance must not delay access, move active targets or conceal needed information.
- Avoid competing cascades in one view. Related groups may overlap when that choreography clarifies the same change.

## Simultaneity

- Unrelated changes do not animate at the same time; detect two independent regions moving together and drawing attention in two directions.
- Related changes share coordinated timing. Their easing and phase may differ by role: the container, departing copy and arriving detail need not move on one identical curve.

## Prototype, don't board

- Temporal hierarchy cannot be judged from still images. Every sequence above, and every expressive moment, is reviewed as a running prototype on a real page, at real speed and at deliberately slowed speed, in the full and the reduced state.
- Detect a motion decision accepted from static boards, annotated frames or duration tables alone; it is not reviewed.
- Prototypes exercise interruption: reverse each step mid-flight and fire the next level early (see [Keep motion responsive when its input changes](interruption-reversal.md)).

## Check the sequence in use

Watch only the order of events and check that it makes cause and effect clear. Dismiss the surface and confirm that control returns promptly; justify any unusual timing by its purpose. Remove cascades where order carries no meaning. Walk the ladder from rest to command and back: each level should appear only when caused, and the page should settle back to reading. Judge each sequence in a running prototype and put the result in its existing review.

## Sources

- [Carbon Design System: Motion choreography](https://carbondesignsystem.com/elements/motion/choreography/): sequencing related elements, what leads and what follows, and staggering only where order carries meaning.
- [Carbon Design System: Motion](https://carbondesignsystem.com/elements/motion/overview/): reserving expressive motion for occasional, important moments as a rhythmic break in productive work.
- [Fluent 2: Motion](https://fluent2.microsoft.design/motion): enter and exit, elevation and container transitions, and scaling timing to size and distance.
- [Material Design 3: Transition patterns](https://m3.material.io/styles/motion/transitions/transition-patterns): container transform, shared axis and fade-through as named patterns for container-before-content and leaving-before-arriving.
- [Apple HIG: Motion](https://developer.apple.com/design/human-interface-guidelines/motion): motion that conveys status, feedback and relationship without getting in the way, and when to leave it out.
